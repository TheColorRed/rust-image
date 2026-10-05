use crate::AbraImage;

/// Non-live edits shared by full-resolution replay and thumbnail workers.
#[derive(Clone, Debug, uniffi::Enum)]
pub enum ImageOperation {
  Rotate { degrees: f64 },
  FlipHorizontal,
  FlipVertical,
  AutoTone,
  AutoColor,
  Posterize { levels: u8 },
  Sharpen,
  Smooth,
}

#[uniffi::export]
impl AbraImage {
  /// Applies a non-live edit using the same implementation at every image size.
  pub fn apply_operation(&self, operation: ImageOperation) {
    match operation {
      ImageOperation::Rotate { degrees } => self.rotate(degrees),
      ImageOperation::FlipHorizontal => self.flip_horizontal(),
      ImageOperation::FlipVertical => self.flip_vertical(),
      ImageOperation::AutoTone => self.auto_tone(),
      ImageOperation::AutoColor => self.auto_color(),
      ImageOperation::Posterize { levels } => self.posterize(levels),
      ImageOperation::Sharpen => self.sharpen(),
      ImageOperation::Smooth => self.smooth(),
    }
  }
}
