use std::sync::{Arc, Mutex, OnceLock, mpsc};

use crate::AbraError;

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
    use crate::{AbraImage, effect_spec::EffectSpec, image_operation::ImageOperation};
    let pixels: Vec<u8> = (0..32 * 16).flat_map(|i| [(i % 255) as u8, 80, 120, 255]).collect();
    let image = AbraImage::from_rgba(32, 16, pixels.clone()).unwrap();
    let source = futures::executor::block_on(image.thumbnail_source_async(16, 16)).unwrap();
    assert_eq!((source.width(), source.height()), (16, 8));
    assert_eq!(image.rgba(), pixels);
    let original = source.rgba();
    for operation in [
      ImageOperation::Rotate { degrees: -90.0 },
      ImageOperation::Rotate { degrees: 90.0 },
      ImageOperation::FlipHorizontal,
      ImageOperation::FlipVertical,
      ImageOperation::AutoTone,
      ImageOperation::AutoColor,
      ImageOperation::Posterize { levels: 6 },
      ImageOperation::Sharpen,
      ImageOperation::Smooth,
    ] {
      let expected = AbraImage::from_image(source.clone_image());
      expected.apply_operation(operation.clone());
      let thumbnail = futures::executor::block_on(source.render_thumbnail_async(vec![], Some(operation))).unwrap();
      assert_eq!(thumbnail.rgba(), expected.rgba());
      assert_eq!((thumbnail.width(), thumbnail.height()), (expected.width(), expected.height()));
    }
    let effects = vec![EffectSpec::Brightness { amount: 20 }, EffectSpec::Invert];
    let expected = AbraImage::from_image(source.clone_image());
    for effect in &effects {
      expected.apply_effect(effect.clone());
    }
    let thumbnail = futures::executor::block_on(source.render_thumbnail_async(effects, None)).unwrap();
    assert_eq!(thumbnail.rgba(), expected.rgba());
    assert_eq!(source.rgba(), original);
    assert!(futures::executor::block_on(image.thumbnail_source_async(0, 16)).is_err());
  }

  #[test]
  fn async_decode_reports_file_errors() {
    let error =
      futures::executor::block_on(crate::AbraImage::read_async("missing-thumbnail-test-image.png".to_owned()))
        .err()
        .expect("missing image should fail");
    assert!(matches!(error, AbraError::Io { .. }));
  }
}
