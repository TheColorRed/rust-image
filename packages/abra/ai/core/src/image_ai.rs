//! What the AI crates that take a picture share.

use crate::{AiError, ControlParams, LabelMap};
use abra_core::{Image, Rect};
use mask::Mask;
use std::path::Path;

/// A model that is loaded from an `.onnx` file and processes a picture.
///
/// The crates that wrap [`AiModel`](crate::AiModel) with a manifest of their own (face landmarks, face parsing, body
/// segmentation, person finding) and the image-to-image [`ImageModel`](crate::ImageModel) all work this way. They differ
/// in what comes out, so that is [`Output`](Self::Output).
///
/// ```ignore
/// let output = FaceParser::load("models/resnet18.onnx")?.process(&photo)?;
/// ```
pub trait ImageAi: Sized {
  /// What processing a picture returns.
  type Output;

  /// Loads the model from its `.onnx` file. The library never looks for models, so the caller gives the path.
  ///
  /// - `p_model_path`: Path to the ONNX model file.
  fn load(p_model_path: impl AsRef<Path>) -> Result<Self, AiError>;

  /// Processes a picture.
  ///
  /// - `p_image`: The picture to process.
  fn process(&self, p_image: &Image) -> Result<Self::Output, AiError>;

  /// Processes only a part of a picture, and gives the result as if the model had been run on the whole picture.
  ///
  /// A model sees a small copy of what it is given, so a person who is a small part of a photo is only a few pixels to it, and the
  /// result is rough. Giving it just the part that matters, with a margin round it, gives it much more to work with. The part is cut
  /// out, processed, and the result is put back where the part was: pixels outside it are `0` in a mask and have no class in labels.
  ///
  /// - `p_image`: The whole picture.
  /// - `p_region`: The part to process, in the pixels of the picture, such as the bounds of a person found by a matting model.
  /// - `p_margin`: How much more to take on every side, as a share of the larger side of the region. `0.1` is a tenth. The part is
  ///   kept inside the picture.
  fn process_within(&self, p_image: &Image, p_region: Rect, p_margin: f32) -> Result<Self::Output, AiError>
  where
    Self::Output: Placeable,
  {
    let (width, height) = p_image.dimensions::<u32>();
    let (left, top, right, bottom) = p_region.edges::<f32>();
    let margin = p_margin.max(0.0) * (right - left).max(bottom - top);
    let (x0, y0) = ((left - margin).max(0.0).floor() as u32, (top - margin).max(0.0).floor() as u32);
    let (x1, y1) = ((right + margin).min(width as f32).ceil() as u32, (bottom + margin).min(height as f32).ceil() as u32);
    if x1 <= x0 || y1 <= y0 {
      return Err(AiError::invalid_input("The region is outside the picture"));
    }
    let mut part = p_image.clone();
    abra_core::crop(x0, y0, x1 - x0, y1 - y0).apply(&mut part);
    Ok(self.process(&part)?.placed(width, height, x0, y0))
  }
}

/// What a model finds in a part of a picture, which can be put back where that part was in the whole picture. It is what
/// [`ImageAi::process_within`] needs of an output.
pub trait Placeable: Sized {
  /// This result for a part of a picture, as a result for the whole picture.
  ///
  /// - `p_width`, `p_height`: The size of the whole picture.
  /// - `p_x`, `p_y`: Where the top left corner of the part is in the whole picture.
  fn placed(&self, p_width: u32, p_height: u32, p_x: u32, p_y: u32) -> Self;
}

impl Placeable for Mask {
  /// The mask is `0` outside the part.
  fn placed(&self, p_width: u32, p_height: u32, p_x: u32, p_y: u32) -> Self {
    Mask::placed(self, p_width, p_height, (p_x as f32, p_y as f32))
  }
}

impl Placeable for LabelMap {
  /// The pixels outside the part have no class ([`LabelMap::NO_CLASS`]).
  fn placed(&self, p_width: u32, p_height: u32, p_x: u32, p_y: u32) -> Self {
    LabelMap::placed(self, p_width, p_height, p_x, p_y)
  }
}

/// An [`ImageAi`] model that also takes control values, such as how strongly to enhance.
pub trait ImageAiWithControl: ImageAi {
  /// Processes a picture with control values, one for each of the model's controls.
  ///
  /// - `p_image`: The picture to process.
  /// - `p_control`: The control values.
  fn process_with_control(&self, p_image: &Image, p_control: &ControlParams) -> Result<Self::Output, AiError>;
}

#[cfg(test)]
mod tests {
  use super::*;

  /// A model that finds everything: a mask of `255` the size of what it is given.
  struct Everything;

  impl ImageAi for Everything {
    type Output = Mask;

    fn load(_p_model_path: impl AsRef<Path>) -> Result<Self, AiError> {
      Ok(Everything)
    }

    fn process(&self, p_image: &Image) -> Result<Mask, AiError> {
      let (width, height) = p_image.dimensions::<u32>();
      Ok(Mask::from_values(width, height, vec![255; (width * height) as usize]))
    }
  }

  /// A model that labels every pixel of what it is given as class 3.
  struct Classes;

  impl ImageAi for Classes {
    type Output = LabelMap;

    fn load(_p_model_path: impl AsRef<Path>) -> Result<Self, AiError> {
      Ok(Classes)
    }

    fn process(&self, p_image: &Image) -> Result<LabelMap, AiError> {
      let (width, height) = p_image.dimensions::<u32>();
      Ok(LabelMap {
        width,
        height,
        labels: vec![3; (width * height) as usize],
        classes: vec!["a".to_string()],
      })
    }
  }

  #[test]
  fn a_mask_made_in_a_part_is_put_back_where_the_part_was() {
    let photo = Image::new(40, 30);
    let mask = Everything.process_within(&photo, Rect::new((10, 8), (10, 6)), 0.0).unwrap();
    assert_eq!(mask.dimensions::<u32>(), (40, 30));
    // Inside the region it is found, and outside it is not.
    assert_eq!((mask.get(10, 8), mask.get(19, 13)), (Some(255), Some(255)));
    assert_eq!((mask.get(9, 8), mask.get(20, 13), mask.get(10, 7), mask.get(10, 14)), (Some(0), Some(0), Some(0), Some(0)));
    assert_eq!(mask.bounds(0).map(|bounds| bounds.edges::<u32>()), Some((10, 8, 20, 14)));
  }

  #[test]
  fn a_margin_takes_more_round_the_region_and_stays_inside_the_picture() {
    let photo = Image::new(40, 30);
    // A tenth of the larger side (10) is one pixel on each side.
    let mask = Everything.process_within(&photo, Rect::new((10, 8), (10, 6)), 0.1).unwrap();
    assert_eq!(mask.bounds(0).map(|bounds| bounds.edges::<u32>()), Some((9, 7, 21, 15)));
    // A region at the edge is kept inside the picture.
    let mask = Everything.process_within(&photo, Rect::new((0, 0), (10, 10)), 0.5).unwrap();
    assert_eq!(mask.bounds(0).map(|bounds| bounds.edges::<u32>()), Some((0, 0, 15, 15)));
  }

  #[test]
  fn labels_found_in_a_part_have_no_class_outside_it() {
    let photo = Image::new(20, 10);
    let labels = Classes.process_within(&photo, Rect::new((5, 2), (6, 4)), 0.0).unwrap();
    assert_eq!((labels.width, labels.height), (20, 10));
    assert_eq!(labels.label_at(5, 2), Some(3));
    assert_eq!(labels.label_at(10, 5), Some(3));
    assert_eq!(labels.label_at(4, 2), Some(LabelMap::NO_CLASS));
    assert_eq!(labels.label_at(11, 5), Some(LabelMap::NO_CLASS));
    // A mask of the class is only where it was found.
    assert_eq!(labels.mask(3).bounds(0).map(|bounds| bounds.edges::<u32>()), Some((5, 2, 11, 6)));
    assert_eq!(labels.classes, vec!["a".to_string()]);
  }

  #[test]
  fn a_region_outside_the_picture_is_an_error() {
    let photo = Image::new(20, 10);
    assert!(Everything.process_within(&photo, Rect::new((50, 50), (5, 5)), 0.0).is_err());
  }
}
