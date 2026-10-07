use abra_core::{Image, ImageRef};

pub trait Tool: Sized {
  // fn options_mut(&mut self) -> &mut Options;
  /// Apply the tool to the current context.
  #[allow(unused_variables)]
  fn apply<'a>(&self, p_image: impl Into<ImageRef<'a>>) {}

  /// Apply the tool returning a image.
  fn create(&self) -> Image {
    Image::new(1, 1)
  }

  // /// Restricts the operation with an optional area or mask.
  // /// - `p_options`: The `ApplyOptions` containing the area and/or mask to use.
  // fn with_options(mut self, p_options: impl Into<Options>) -> Self {
  //   *self.options_mut() = p_options.into();
  //   self
  // }
}
