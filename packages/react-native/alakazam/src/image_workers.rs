use std::sync::{
  Arc, Condvar, Mutex, OnceLock,
  atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering},
  mpsc,
};
use std::time::Duration;

use abra::abra_core::{Rect, TransformAlgorithm};
use abra::mask::prelude as library_mask;
use abra_body_segmentation::prelude::ImageAi;
use abra_ai_core::{EdgeMatteOptions, edge_matte};
use abra_uniffi::{AiModel, AiOutput, BodySegments, Image, Mask, guided_filter};
use abra_body_segmentation::BodyRegion;

use crate::{
  AbraError,
  consts::EDGE_REFINEMENT_ROLE,
  models::{BODY_PARSER, PERSON_SEGMENTER, SKIN_SEGMENTER},
};

/// A small shared flag, changed under a short lock and read without holding that lock during model execution.
struct SingleFlight {
  busy: Mutex<bool>,
  available: Condvar,
}

impl SingleFlight {
  const fn new() -> Self {
    Self {
      busy: Mutex::new(false),
      available: Condvar::new(),
    }
  }

  fn enter(&'static self, state: &ImageWorkState) -> Option<SingleFlightPermit> {
    let mut busy = self.busy.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    while *busy {
      if state.is_cancelled() {
        return None;
      }
      busy = self
        .available
        .wait_timeout(busy, Duration::from_millis(100))
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .0;
    }
    if state.is_cancelled() {
      return None;
    }
    *busy = true;
    drop(busy);
    Some(SingleFlightPermit(self))
  }
}

struct SingleFlightPermit(&'static SingleFlight);

impl Drop for SingleFlightPermit {
  fn drop(&mut self) {
    let mut busy = self.0.busy.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    *busy = false;
    self.0.available.notify_one();
  }
}

// Keep high-memory edge refinements sequential, even when different editor sessions request them at once.
static EDGE_MATTE_SINGLE_FLIGHT: SingleFlight = SingleFlight::new();

/// Progress and cancellation shared with JavaScript through cheap atomic operations.
#[derive(uniffi::Object)]
pub struct ImageWorkState {
  share: AtomicU32,
  cancelled: AtomicBool,
}

#[uniffi::export]
impl ImageWorkState {
  #[uniffi::constructor]
  pub fn new() -> Self {
    Self {
      share: AtomicU32::new(0.0f32.to_bits()),
      cancelled: AtomicBool::new(false),
    }
  }

  /// The completed share of edge-matte tiles, from `0.0` to `1.0`.
  pub fn share(&self) -> f32 {
    f32::from_bits(self.share.load(Ordering::Relaxed))
  }

  /// Stops the current native image job at its next safe checkpoint.
  pub fn cancel(&self) {
    self.cancelled.store(true, Ordering::Relaxed);
  }
}

impl ImageWorkState {
  fn is_cancelled(&self) -> bool {
    self.cancelled.load(Ordering::Relaxed)
  }
}

/// One person found in a photo: where they are in the photo's pixels, how sure the model is, and the mask of their body.
#[derive(uniffi::Record)]
pub struct PersonDetection {
  pub id: u32,
  pub x: u32,
  pub y: u32,
  pub width: u32,
  pub height: u32,
  pub confidence: f64,
  pub mask: Arc<Mask>,
}

type Job = Box<dyn FnOnce() + Send>;

/// The stack of each worker thread, in megabytes, unless the app chose another size (see [`set_worker_stack_megabytes`]). The default
/// (2 MB) is too small for the AI jobs: a deep call chain in a model run overflowed it and crashed the app with a stack overflow. The
/// memory is only reserved, and used as far as a job goes.
const DEFAULT_WORKER_STACK_MEGABYTES: usize = 16;
/// The smallest and largest stack the app may choose, in megabytes.
const WORKER_STACK_RANGE: std::ops::RangeInclusive<u32> = 2..=256;

static WORKER_STACK_MEGABYTES: AtomicUsize = AtomicUsize::new(DEFAULT_WORKER_STACK_MEGABYTES);

/// Chooses the stack of each image worker thread, in megabytes (kept between 2 and 256). The workers are made once, when the first image
/// job starts, so this has to be called before then: a size chosen after that takes effect the next time the app starts.
#[uniffi::export]
pub fn set_worker_stack_megabytes(megabytes: u32) {
  let megabytes = megabytes.clamp(*WORKER_STACK_RANGE.start(), *WORKER_STACK_RANGE.end());
  WORKER_STACK_MEGABYTES.store(megabytes as usize, Ordering::Relaxed);
}

/// Makes the AI models leave room for the screen. A model run uses as many threads as it is allowed, and on a phone or tablet that is every core
/// by default, which leaves the JavaScript and render threads too little CPU, so the app answers seconds late while the AI works. This gives
/// each model run half of the device's threads (at least one) and stops them spinning while they wait. Call it once when the app starts, before
/// the first model is loaded; models already loaded keep their settings. Returns the number of threads a model run uses.
#[uniffi::export]
pub fn configure_ai_inference() -> u32 {
  let threads = (std::thread::available_parallelism().map_or(2, |threads| threads.get()) / 2).max(1);
  abra_ai_core::prelude::OnnxConfig::set_default_threads(Some(threads));
  abra_ai_core::prelude::OnnxConfig::set_default_spinning(Some(false));
  threads as u32
}

/// How many image jobs (decoding, thumbnails, downloads) run at the same time: half of the threads this device
/// offers, and at least one. Each job also uses Abra's own parallel threads and holds a whole photo in memory.
fn worker_count() -> usize {
  (std::thread::available_parallelism().map_or(2, |threads| threads.get()) / 2).max(1)
}

fn workers() -> Result<&'static mpsc::Sender<Job>, AbraError> {
  static POOL: OnceLock<Result<mpsc::Sender<Job>, String>> = OnceLock::new();
  POOL
    .get_or_init(|| {
      let (sender, receiver) = mpsc::channel::<Job>();
      let receiver = Arc::new(Mutex::new(receiver));
      for index in 0..worker_count() {
        let receiver = Arc::clone(&receiver);
        std::thread::Builder::new()
          .name(format!("abra-image-{index}"))
          .stack_size(WORKER_STACK_MEGABYTES.load(Ordering::Relaxed) * 1024 * 1024)
          .spawn(move || {
            loop {
              let job = receiver.lock().unwrap().recv();
              match job {
                Ok(job) => job(),
                Err(_) => break,
              }
            }
          })
          .map_err(|error| format!("Could not start image worker: {error}"))?;
      }
      Ok(sender)
    })
    .as_ref()
    .map_err(|message| AbraError::Render {
      message: message.clone(),
    })
}

pub(crate) async fn run<T: Send + 'static>(
  p_work: impl FnOnce() -> Result<T, AbraError> + Send + 'static,
) -> Result<T, AbraError> {
  let (sender, receiver) = futures::channel::oneshot::channel();
  workers()?
    .send(Box::new(move || {
      let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(p_work)).unwrap_or_else(|_| {
        eprintln!("[image] image worker panicked");
        Err(AbraError::Render {
          message: "Image worker panicked".to_owned(),
        })
      });
      // A cancelled caller no longer needs the result; dropping it releases native pixels.
      let _ = sender.send(result);
    }))
    .map_err(|_| AbraError::Render {
      message: "Image worker pool stopped".to_owned(),
    })?;
  receiver.await.map_err(|_| AbraError::Render {
    message: "Image worker stopped before returning a result".to_owned(),
  })?
}

/// Defines a function that JavaScript calls with `await`, whose body runs on a worker thread, so the app keeps drawing while it works.
/// This is the whole of what a worker is: its name, its arguments and its body. The pool, the hand-off back to the caller, a panic that
/// becomes an error and the export to JavaScript are done here, once.
///
/// The body returns a `Result<_, AbraError>`, and `?` works on anything an Abra AI crate returns. The arguments are moved onto the
/// worker, so an image arrives as the handle and is turned into pixels with `.core()`, which only shares them.
///
/// ```ignore
/// worker! {
///   /// The mask of the skin in the photo.
///   detect_skin_async(image: Arc<Image>) -> Arc<Mask> {
///     Ok(Arc::new(Mask::from_core(SKIN_SEGMENTER.get()?.process(&image.core())?)))
///   }
/// }
/// ```
macro_rules! worker {
  ($(#[$meta:meta])* $name:ident($($arg:ident: $ty:ty),* $(,)?) -> $out:ty $body:block) => {
    $(#[$meta])*
    #[uniffi::export]
    pub async fn $name($($arg: $ty),*) -> Result<$out, AbraError> {
      run(move || $body).await
    }
  };
}

worker! {
  /// The mask of the skin in the photo.
  detect_skin_async(image: Arc<Image>) -> Arc<Mask> {
    Ok(Arc::new(Mask::from_core(SKIN_SEGMENTER.get()?.process(&image.core())?)))
  }
}

worker! {
  /// The people in the photo, each with the mask of their body.
  find_people_async(image: Arc<Image>) -> Vec<PersonDetection> {
    let (width, height) = (image.width(), image.height());
    let people = PERSON_SEGMENTER.get()?.process(&image.core())?;
    Ok(people.iter().enumerate().map(|(id, person)| {
      let bounds = person.bounds_in(width, height);
      PersonDetection {
        id: id as u32, x: bounds.x, y: bounds.y, width: bounds.width, height: bounds.height,
        confidence: person.confidence as f64, mask: Arc::new(Mask::from_core(person.mask.clone())),
      }
    }).collect())
  }
}

worker! {
  /// Joins person masks into one photo-sized mask using bilinear resizing.
  maximum_masks_async(masks: Vec<Arc<Mask>>, width: u32, height: u32) -> Option<Arc<Mask>> {
    let mut joined = library_mask::Mask::empty(width, height);
    for mask in masks {
      let resized = mask.core().resized(width, height, TransformAlgorithm::Bilinear);
      joined = joined.maximum(&resized);
    }
    Ok(joined.bounds(0).map(|_| Arc::new(Mask::from_core(joined))))
  }
}

worker! {
  /// Runs guided edge matting and tile assembly on a native worker, serializing the high-memory model work.
  edge_matte_async(image: Arc<Image>, rough: Arc<Mask>, state: Arc<ImageWorkState>) -> Option<Arc<Mask>> {
    let Some(_single_flight) = EDGE_MATTE_SINGLE_FLIGHT.enter(&state) else { return Ok(None) };
    let path = crate::models::ai_model_path(EDGE_REFINEMENT_ROLE.to_owned())?;
    let model = AiModel::load_native(path).map_err(|error| AbraError::Ai { message: error.to_string() })?;
    if state.is_cancelled() {
      return Ok(None);
    }
    let refined = edge_matte(
      model.core(),
      &image.core(),
      &rough.core(),
      EdgeMatteOptions::default(),
      |share| state.share.store(share.to_bits(), Ordering::Relaxed),
      || state.is_cancelled(),
    )?;
    Ok(refined.map(|mask| Arc::new(Mask::from_core(mask))))
  }
}

worker! {
  /// Parses, smooths and fits the body regions against a cutout on a native worker.
  fit_body_parts_async(
    image: Arc<Image>,
    cutout: Arc<Mask>,
    margin: f32,
    follow: f32,
    epsilon: f32,
    state: Arc<ImageWorkState>,
  ) -> Vec<Arc<Mask>> {
    if state.is_cancelled() { return Ok(Vec::new()) }
    let photo = image.core();
    let cutout = cutout.core();
    let Some(bounds) = cutout.bounds(128) else { return Ok(Vec::new()) };
    let segments = BODY_PARSER.get()?.process_within(&photo, bounds, margin)?;
    if state.is_cancelled() { return Ok(Vec::new()) }
    let radius = ((bounds.width * bounds.height).sqrt() * follow).round().max(2.0) as u32;
    let regions = [
      segments.mask(BodyRegion::Hair),
      segments.mask_of(&[BodyRegion::BodySkin, BodyRegion::FaceSkin]),
      segments.mask(BodyRegion::Clothes),
      segments.mask(BodyRegion::Accessories),
    ];
    let mut followed = Vec::with_capacity(regions.len());
    for region in regions {
      if state.is_cancelled() { return Ok(Vec::new()) }
      followed.push(guided_filter(Arc::new(Mask::from_core(region)), Arc::new(Image::from_core(photo.clone())), radius, epsilon).core());
    }
    let whole = cutout.maximum(&followed[3]);
    let mut output = Vec::with_capacity(5);
    output.push(Arc::new(Mask::from_core(whole.clone())));
    output.extend(followed.iter().map(|part| Arc::new(Mask::from_core(part.multiplied(&whole)))));
    Ok(output)
  }
}

worker! {
  /// Labels the hair, skin, clothes and accessories of the person in the photo.
  parse_body_async(image: Arc<Image>) -> Arc<BodySegments> {
    Ok(Arc::new(BodySegments::from_core(BODY_PARSER.get()?.process(&image.core())?)))
  }
}

worker! {
  /// Labels the body in only a part of the photo, the rectangle at `x`, `y` of `width` by `height` pixels (the person's bounds, say), with
  /// a `margin` of more on each side as a share of the part's
  /// larger side. The model sees a small copy of what it is given, so a person who is small in the photo is far better found this way.
  /// Pixels outside the part have no region.
  parse_body_within_async(image: Arc<Image>, x: u32, y: u32, width: u32, height: u32, margin: f32) -> Arc<BodySegments> {
    let region = Rect::new((x, y), (width, height));
    Ok(Arc::new(BodySegments::from_core(BODY_PARSER.get()?.process_within(&image.core(), region, margin)?)))
  }
}

worker! {
  /// Runs an AI model on a picture on a worker thread, so the app keeps drawing while it runs. The same as `AiModel.run`.
  run_ai_model_async(model: Arc<AiModel>, image: Arc<Image>) -> Arc<AiOutput> {
    Ok(Arc::new(AiOutput::from_core(model.core().run(&image.core())?)))
  }
}

worker! {
  /// Loads a model and its manifest on an image worker thread, keeping large model initialization off the UI thread.
  load_ai_model_async(role: String) -> Arc<AiModel> {
    let path = crate::models::ai_model_path(role)?;
    AiModel::load_native(path).map(Arc::new).map_err(|error| AbraError::Ai { message: error.to_string() })
  }
}

worker! {
  /// Runs an AI model that takes a guide mask (a matting model and its trimap) on a worker thread. The same as `AiModel.run_guided`.
  run_ai_model_guided_async(model: Arc<AiModel>, image: Arc<Image>, guide: Arc<Mask>) -> Arc<AiOutput> {
    Ok(Arc::new(AiOutput::from_core(model.core().run_guided(&image.core(), Some(&guide.core()), &|_| {})?)))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn image_work_is_parallel_bounded_and_off_the_calling_thread() {
    let caller = std::thread::current().id();
    let release = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
    let (sender, receiver) = mpsc::channel();
    let jobs = (0..worker_count() + 1)
      .map(|_| {
        let release = Arc::clone(&release);
        let sender = sender.clone();
        run(move || {
          assert_ne!(std::thread::current().id(), caller);
          sender.send(std::thread::current().name().unwrap().to_owned()).unwrap();
          let (released, wake) = &*release;
          let _guard = wake.wait_while(released.lock().unwrap(), |released| !*released).unwrap();
          Ok(())
        })
      })
      .collect::<Vec<_>>();
    let runner = std::thread::spawn(move || futures::executor::block_on(futures::future::join_all(jobs)));
    // One job per worker starts at once, each on its own thread.
    let started: Vec<String> =
      (0..worker_count()).map(|_| receiver.recv_timeout(std::time::Duration::from_secs(5)).unwrap()).collect();
    // With every worker busy, the extra job has to wait.
    let extra = receiver.recv_timeout(std::time::Duration::from_millis(200));
    *release.0.lock().unwrap() = true;
    release.1.notify_all();
    let results = runner.join().unwrap();
    let mut distinct = started.clone();
    distinct.sort();
    distinct.dedup();
    assert_eq!(distinct.len(), worker_count(), "each worker runs one job at a time");
    assert!(started.iter().all(|name| name.starts_with("abra-image-")));
    assert!(extra.is_err(), "the pool must never run more than {} jobs at once", worker_count());
    assert!(results.into_iter().all(|result| result.is_ok()));
  }
}
