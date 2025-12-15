use crate::ControlParams;
use abra_core::Image;

/// This trait is used to implement how a tool loads a model for use.
/// This is the base implementation, then one or both of `AiProcessModel` or `AiProcessModelControl`
/// can be implemented depending on whether the tool needs to support control parameters.
pub trait AiModel {
  /// This trait is used to implement how a tool loads a model for use.
  ///
  /// # Arguments
  ///
  /// - `p_name`: The name of the model to load.
  fn load(p_name: impl AsRef<str>) -> Self;
}
/// This trait is used to implement how a tool processes an input image.
/// Implement this if the tool doesn't need to support control parameters.
pub trait AiProcessModel {
  /// This function describes how an AI tool should process an input image.
  ///
  /// # Arguments
  ///
  /// - `p_image`: The input image to be processed.
  /// - `ctrl`: Optional control parameters. Pass `None`, `ControlParams`, or `&ControlParams`.
  ///
  /// # Example
  ///
  /// ```ignore
  /// let image = Image::new_from_path("input.png");
  /// let model = MyAiModel::load("my-model");
  /// let output = model.process(&image);
  /// output.save("output.png", None);
  /// ```
  fn process(&self, p_image: &Image) -> Image;
}
/// This trait is used to implement how a tool processes an input image with control parameters.
/// Implement this if the tool needs to support control parameters.
pub trait AiProcessModelControl {
  /// This function describes how an AI tool should process an input image with control parameters.
  ///
  /// # Arguments
  ///
  /// - `p_image`: The input image to be processed.
  /// - `p_ctrl`: Control parameters.
  ///
  /// # Example
  ///
  /// ```ignore
  /// let image = Image::new_from_path("input.png");
  /// let ctrl = ControlParams::new(&[0.3, 0.5, 0.7]);
  /// let model = MyAiModel::load("my-model");
  /// let output = model.process_with_control(&image, &ctrl);
  /// output.save("output.png", None);
  /// ```
  fn process_with_control(&self, p_image: &Image, p_ctrl: &ControlParams) -> Image;
}
