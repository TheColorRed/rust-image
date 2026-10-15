//! Progress reporting for models that process an image in steps.

use crate::error::AiError;
use crate::image_ai::ImageAi;
use abra_core::Image;

/// A model that can tell how far along it is while it processes an image.
///
/// A model implements [`process_reporting`](Self::process_reporting), and [`on`](Self::on) comes with it:
///
/// ```ignore
/// let output = model.on(|progress| println!("{progress}")).process(&image)?;
/// ```
pub trait ReportsProgress: ImageAi {
  /// Processes an image, calling `p_progress` with a value from `0.0` to `1.0`: `0.0` when processing starts, then the
  /// share finished after each step, ending on `1.0`.
  fn process_reporting(&self, p_input: &Image, p_progress: &dyn Fn(f32)) -> Result<Self::Output, AiError>;

  /// Listens to the progress of what the model processes next. The listener belongs to the returned [`Listening`], so it
  /// hears only what that processes.
  fn on<'a>(&'a self, p_listener: impl Fn(f32) + 'a) -> Listening<'a, Self>
  where
    Self: Sized,
  {
    Listening {
      model: self,
      listener: Box::new(p_listener),
    }
  }
}

/// A model with a listener for the progress of what it processes, from [`ReportsProgress::on`].
pub struct Listening<'a, M> {
  pub(crate) model: &'a M,
  pub(crate) listener: Box<dyn Fn(f32) + 'a>,
}

impl<M: ReportsProgress> Listening<'_, M> {
  /// Processes an image like the model's own `process`, telling the listener how far along it is.
  pub fn process(&self, p_input: &Image) -> Result<M::Output, AiError> {
    self.model.process_reporting(p_input, &*self.listener)
  }
}
