use abra_core::ImageRef;

pub trait Tool: Sized {
  // fn options_mut(&mut self) -> &mut Options;
  /// Apply the tool to the current context.
  fn apply<'a>(&self, p_image: impl Into<ImageRef<'a>>);

  // /// Restricts the operation with an optional area or mask.
  // /// - `p_options`: The `ApplyOptions` containing the area and/or mask to use.
  // fn with_options(mut self, p_options: impl Into<Options>) -> Self {
  //   *self.options_mut() = p_options.into();
  //   self
  // }
}
