use std::sync::{Arc, Mutex, OnceLock, mpsc};

use abra::prelude::{Image as CoreImage, ImageExt};
use abra_body_segmentation::prelude::ImageAi;
use abra_uniffi::{AiModel, AiOutput, Image, Mask};

use crate::{AbraError, models};

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

/// The mask of the skin in the photo.
#[uniffi::export]
pub async fn detect_skin_async(image: Arc<Image>) -> Result<Arc<Mask>, AbraError> {
  let pixels = image.core();
  run(move || {
    let mask = models::skin_segmenter()?.process(&pixels).map_err(|error| AbraError::Ai { message: error.to_string() })?;
    Ok(Arc::new(Mask::from_core(mask)))
  }).await
}

/// The people in the photo, each with the mask of their body.
#[uniffi::export]
pub async fn find_people_async(image: Arc<Image>) -> Result<Vec<PersonDetection>, AbraError> {
  let pixels = image.core();
  let (width, height) = (image.width(), image.height());
  run(move || {
    let people = models::person_segmenter()?.process(&pixels).map_err(|error| AbraError::Ai { message: error.to_string() })?;
    Ok(people.iter().enumerate().map(|(id, person)| {
      let bounds = person.bounds_in(width, height);
      PersonDetection {
        id: id as u32, x: bounds.x, y: bounds.y, width: bounds.width, height: bounds.height,
        confidence: person.confidence as f64, mask: Arc::new(Mask::from_core(person.mask.clone())),
      }
    }).collect())
  }).await
}

/// Runs an AI model on a picture on a worker thread, so the app keeps drawing while it runs. The same as `AiModel.run`.
#[uniffi::export]
pub async fn run_ai_model_async(model: Arc<AiModel>, image: Arc<Image>) -> Result<Arc<AiOutput>, AbraError> {
  let pixels = image.core();
  run(move || {
    let output = model.core().run(&pixels).map_err(|error| AbraError::Ai { message: error.to_string() })?;
    Ok(Arc::new(AiOutput::from_core(output)))
  }).await
}

/// Runs an AI model that takes a guide mask (a matting model and its trimap) on a worker thread. The same as `AiModel.run_guided`.
#[uniffi::export]
pub async fn run_ai_model_guided_async(
  model: Arc<AiModel>, image: Arc<Image>, guide: Arc<Mask>,
) -> Result<Arc<AiOutput>, AbraError> {
  let (pixels, guide) = (image.core(), guide.core());
  run(move || {
    let output =
      model.core().run_guided(&pixels, Some(&guide), &|_| {}).map_err(|error| AbraError::Ai { message: error.to_string() })?;
    Ok(Arc::new(AiOutput::from_core(output)))
  }).await
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
}
