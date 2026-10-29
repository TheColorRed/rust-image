use std::sync::{
  Arc, Mutex, OnceLock,
  atomic::{AtomicUsize, Ordering},
  mpsc,
};

use abra::abra_core::Rect;
use abra::prelude::{Image as CoreImage, ImageExt};
use abra_body_segmentation::prelude::ImageAi;
use abra_uniffi::{AiModel, AiOutput, BodySegments, Canvas, Image, Mask};

use crate::{
  AbraError,
  models::{BODY_PARSER, PERSON_SEGMENTER, SKIN_SEGMENTER},
};

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

/// One cell of a canvas made by `canvas_of_cells_async`: the picture file that goes in it, and where the cell is and how big it is, in pixels
/// of the canvas. The library's own `CanvasCell` (see `Canvas.new_of_cells`) as a record of this crate, since a record can only be sent
/// across by the crate that declares it.
#[derive(uniffi::Record)]
pub struct PictureCell {
  pub path: String,
  pub x: i32,
  pub y: i32,
  pub width: u32,
  pub height: u32,
  /// Keeps the picture's own proportions, with nothing cropped, and puts the cell in the middle of its place. See `CanvasCell`.
  pub keep_aspect: bool,
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
  /// Reads an image file on a worker thread.
  read_image_async(path: String) -> Arc<Image> {
    CoreImage::read(path).map(Image::from_core).map(Arc::new).map_err(|message| AbraError::Io { message })
  }
}

worker! {
  /// A canvas of `width` x `height` with a child canvas for each of `cells`, each holding its picture scaled to cover the cell, made on a
  /// worker thread so the app keeps drawing while the pictures are read and scaled. The same as `Canvas.new_of_cells`, which says how.
  canvas_of_cells_async(name: String, width: u32, height: u32, cells: Vec<PictureCell>) -> Arc<Canvas> {
    let cells: Vec<_> = cells
      .into_iter()
      .map(|cell| abra::canvas::prelude::CanvasCell { path: cell.path, x: cell.x, y: cell.y, width: cell.width, height: cell.height, keep_aspect: cell.keep_aspect })
      .collect();
    abra::canvas::prelude::Canvas::new_of_cells(name, width, height, &cells)
      .map(|canvas| Arc::new(Canvas::from_core(canvas)))
      .map_err(|message| AbraError::Io { message })
  }
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
    AiModel::load(path).map(Arc::new).map_err(|error| AbraError::Ai { message: error.to_string() })
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

  #[test]
  fn async_decode_reports_file_errors() {
    let error =
      futures::executor::block_on(read_image_async("missing-thumbnail-test-image.png".to_owned()))
        .err()
        .expect("missing image should fail");
    assert!(matches!(error, AbraError::Io { .. }));
  }

  #[test]
  fn a_canvas_of_cells_is_made_on_a_worker_and_a_missing_picture_is_an_error() {
    let path = std::env::temp_dir().join(format!("abra-cells-worker-{}.png", std::process::id()));
    let picture = CoreImage::new_from_color(8, 6, abra::abra_core::Color::from_rgba(90, 90, 90, 255));
    abra::abra_core::writer(path.to_string_lossy().into_owned()).save(&picture).unwrap();
    let cell = |path: &str, x| PictureCell { path: path.to_owned(), x, y: 0, width: 4, height: 4, keep_aspect: false };
    let file = path.to_string_lossy().into_owned();

    let canvas = futures::executor::block_on(canvas_of_cells_async(
      "Cells".to_owned(), 8, 4, vec![cell(&file, 0), cell(&file, 4)],
    ))
    .expect("two cells of one file");
    assert_eq!(canvas.canvases().len(), 2);
    assert_eq!(canvas.as_image().core().get_pixel(5, 1).unwrap().0, 90);

    let error = futures::executor::block_on(canvas_of_cells_async(
      "Cells".to_owned(), 8, 4, vec![cell("missing-cell-picture.png", 0)],
    ))
    .err()
    .expect("a missing picture is an error");
    assert!(matches!(error, AbraError::Io { .. }));
    let _ = std::fs::remove_file(path);
  }
}
