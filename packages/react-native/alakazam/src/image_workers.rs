use std::sync::{Arc, Mutex, OnceLock, mpsc};

use abra::prelude::{Image as CoreImage, ImageExt};
use abra_uniffi::{Image, PersonDetection};

use crate::{AbraError, effect_spec::{EffectSpec, apply_effect}, models};

type Job = Box<dyn FnOnce() + Send>;

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

#[uniffi::export]
pub async fn read_image_async(path: String) -> Result<Arc<Image>, AbraError> {
  run(move || CoreImage::read(path).map(Image::from_core).map(Arc::new).map_err(|message| AbraError::Io { message })).await
}

#[uniffi::export]
pub async fn read_image_with_project_resources_async(
  path: String,
  skin_mask_path: Option<String>,
  people_dir: Option<String>,
  people: Option<Vec<PersonDetection>>,
) -> Result<Arc<Image>, AbraError> {
  run(move || Ok(Arc::new(Image::read_with_project_resources(path, skin_mask_path, people_dir, people)))).await
}

#[uniffi::export]
pub async fn thumbnail_source_async(image: Arc<Image>, width: u32, height: u32) -> Result<Arc<Image>, AbraError> {
  if width == 0 || height == 0 {
    return Err(AbraError::InvalidPixels { message: "Thumbnail dimensions must be positive".to_owned() });
  }
  run(move || Ok(image.shrunk(width, height))).await
}

#[uniffi::export]
pub async fn render_thumbnail_async(image: Arc<Image>, effects: Vec<EffectSpec>) -> Result<Arc<Image>, AbraError> {
  run(move || {
    let rendered = image.duplicate();
    for effect in effects { apply_effect(Arc::clone(&rendered), effect); }
    Ok(rendered)
  }).await
}

#[uniffi::export]
pub async fn detect_skin_async(image: Arc<Image>) -> Result<(), AbraError> {
  if image.base_skin_mask().is_some() { return Ok(()) }
  let pixels = image.core();
  run(move || {
    let mask = models::skin_segmenter()?.process(&pixels).map_err(|error| AbraError::Ai { message: error.to_string() })?;
    if image.base_skin_mask().is_none() { image.set_skin_mask(mask); }
    Ok(())
  }).await
}

#[uniffi::export]
pub async fn find_people_async(image: Arc<Image>) -> Result<Vec<PersonDetection>, AbraError> {
  if image.cached_people().is_none() {
    let pixels = image.core();
    let target = Arc::clone(&image);
    let people = run(move || {
      models::person_segmenter()?.process(&pixels).map_err(|error| AbraError::Ai { message: error.to_string() })
    }).await?;
    if target.cached_people().is_none() { target.set_people(people); }
  }
  Ok(image.find_people())
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
  fn async_thumbnail_edits_match_replay_and_leave_the_source_unchanged() {
    use abra::abra_core::Channels;
    use crate::effect_spec::EffectSpec;
    let pixels: Vec<u8> = (0..32 * 16).flat_map(|i| [(i % 255) as u8, 80, 120, 255]).collect();
    let image = Arc::new(Image::from_core(CoreImage::new_from_pixels(32, 16, pixels.clone(), Channels::RGBA)));
    let source = futures::executor::block_on(thumbnail_source_async(Arc::clone(&image), 16, 16)).unwrap();
    assert_eq!((source.width(), source.height()), (16, 8));
    assert_eq!(image.rgba(), pixels);
    let original = source.rgba();
    let effects = vec![EffectSpec::Brightness { amount: 20 }, EffectSpec::Invert];
    let expected = source.duplicate();
    for effect in &effects {
      apply_effect(Arc::clone(&expected), effect.clone());
    }
    let thumbnail = futures::executor::block_on(render_thumbnail_async(Arc::clone(&source), effects)).unwrap();
    assert_eq!(thumbnail.rgba(), expected.rgba());
    assert_eq!(source.rgba(), original);
    assert!(futures::executor::block_on(thumbnail_source_async(image, 0, 16)).is_err());
  }

  #[test]
  fn async_decode_reports_file_errors() {
    let error =
      futures::executor::block_on(read_image_async("missing-thumbnail-test-image.png".to_owned()))
        .err()
        .expect("missing image should fail");
    assert!(matches!(error, AbraError::Io { .. }));
  }
}
