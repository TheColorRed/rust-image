//! YOLO26 instance segmentation for selecting people in an image.
//!
//! [`PersonSegmenter`] is [`AiModel`] with the model's manifest built in, so a caller only needs the `.onnx` file.

use abra_ai_core::{AiError, AiModel, AiOutput, Bounds, Instance};
use abra_core::Image;
pub use mask::Mask;
use std::path::Path;

const MANIFEST: &str = include_str!("../models/yolo26n-seg.yml");

/// The bounds of one detected person, in source-image pixels.
pub type PersonBounds = Bounds;

/// One person instance and its soft, per-pixel silhouette mask.
#[derive(Clone, Debug)]
pub struct PersonInstance {
  pub bounds: PersonBounds,
  pub confidence: f32,
  pub mask: Mask,
}

/// Finds person instances using a YOLO26n segmentation model supplied by the caller.
impl PersonInstance {
  /// The bounds in the pixels of a `p_width` x `p_height` image. People are found in one size of a photo and may be used for
  /// a copy of another size, so the box is taken from the size of this person's own mask, which is the size of the photo they
  /// were found in, to the size asked for. It is rounded outwards and kept inside the image.
  pub fn bounds_in(&self, p_width: u32, p_height: u32) -> PersonBounds {
    let (mask_width, mask_height) = self.mask.dimensions::<u32>();
    let (scale_x, scale_y) = (p_width as f64 / mask_width.max(1) as f64, p_height as f64 / mask_height.max(1) as f64);
    let left = (self.bounds.x as f64 * scale_x).floor();
    let top = (self.bounds.y as f64 * scale_y).floor();
    let right = ((self.bounds.x + self.bounds.width) as f64 * scale_x).ceil().min(p_width as f64);
    let bottom = ((self.bounds.y + self.bounds.height) as f64 * scale_y).ceil().min(p_height as f64);
    PersonBounds {
      x: left as u32,
      y: top as u32,
      width: (right - left) as u32,
      height: (bottom - top) as u32,
    }
  }
}

/// The index of the person a tap at (`p_x`, `p_y`) in a `p_width` x `p_height` image is on, or `None` when it is on no one.
///
/// Bodies can overlap (a baby held by a parent). A person whose outline is under the tap is preferred, then the smallest, so the
/// inner body can be picked. The tap is taken into the frame each person was found in, as in [`PersonInstance::bounds_in`].
pub fn person_at(p_people: &[PersonInstance], p_x: f64, p_y: f64, p_width: u32, p_height: u32) -> Option<usize> {
  p_people
    .iter()
    .enumerate()
    .filter_map(|(index, person)| {
      let (mask_width, mask_height) = person.mask.dimensions::<u32>();
      let (x, y) = (p_x * mask_width as f64 / p_width as f64, p_y * mask_height as f64 / p_height as f64);
      let bounds = person.bounds;
      let inside = x >= bounds.x as f64
        && x < (bounds.x + bounds.width) as f64
        && y >= bounds.y as f64
        && y < (bounds.y + bounds.height) as f64;
      let under_outline =
        person.mask.values().get(y as usize * mask_width as usize + x as usize).is_some_and(|value| *value > 127);
      inside.then_some((index, !under_outline, bounds.width as u64 * bounds.height as u64))
    })
    .min_by_key(|(_, not_under_outline, area)| (*not_under_outline, *area))
    .map(|(index, _, _)| index)
}

pub struct PersonSegmenter {
  model: AiModel,
}

pub mod prelude {
  pub use crate::{PersonBounds, PersonInstance, PersonSegmenter, person_at};
  pub use abra_ai_core::AiError;
  pub use mask::Mask;
}

impl PersonSegmenter {
  /// Loads a YOLO26n-seg ONNX model from `p_model_path`.
  ///
  /// The caller decides where the model lives. It must have this model's input and output layout.
  pub fn load(p_model_path: impl AsRef<Path>) -> Result<Self, AiError> {
    Ok(Self {
      model: AiModel::load_with_manifest(p_model_path, MANIFEST)?,
    })
  }

  /// Finds people in `p_image`, returning source-sized instance masks.
  pub fn process(&self, p_image: &Image) -> Result<Vec<PersonInstance>, AiError> {
    match self.model.run(p_image)? {
      AiOutput::Instances(people) => Ok(
        people
          .into_iter()
          .map(
            |Instance {
               bounds,
               confidence,
               mask,
               ..
             }| PersonInstance {
              bounds,
              confidence,
              mask,
            },
          )
          .collect(),
      ),
      _ => Err(AiError::inference_failed("The person model did not return instances")),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::{Channels, Image};

  /// A person whose box and mask cover `(x, y, w, h)` of a 20 x 20 photo.
  fn body(x: u32, y: u32, w: u32, h: u32) -> PersonInstance {
    let pixels: Vec<u8> = (0..20u32)
      .flat_map(|row| {
        (0..20u32).flat_map(move |col| {
          let value = if (x..x + w).contains(&col) && (y..y + h).contains(&row) { 255 } else { 0 };
          [value, value, value, 255]
        })
      })
      .collect();
    PersonInstance {
      bounds: PersonBounds {
        x,
        y,
        width: w,
        height: h,
      },
      confidence: 0.9,
      mask: Mask::from_image(Image::new_from_pixels(20, 20, pixels, Channels::RGBA)),
    }
  }

  #[test]
  fn a_tap_inside_an_overlapping_body_picks_the_inner_one() {
    let people = [body(0, 0, 20, 20), body(8, 8, 6, 6)];
    assert_eq!(person_at(&people, 10.0, 10.0, 20, 20), Some(1), "the tap is on the inner body");
    assert_eq!(person_at(&people, 2.0, 2.0, 20, 20), Some(0), "the tap is only on the outer body");
    assert_eq!(person_at(&[body(8, 8, 6, 6)], 2.0, 2.0, 20, 20), None, "the tap is on no one");
  }

  #[test]
  fn a_tap_is_taken_into_the_frame_the_person_was_found_in() {
    let people = [body(8, 8, 6, 6)];
    // The photo is now 40 x 40, twice the size the person was found in.
    assert_eq!(person_at(&people, 20.0, 20.0, 40, 40), Some(0));
    assert_eq!(person_at(&people, 4.0, 4.0, 40, 40), None);
  }

  #[test]
  fn bounds_scale_to_another_size_of_the_photo_and_stay_inside_it() {
    let person = body(5, 5, 11, 15);
    assert_eq!(person.bounds_in(20, 20), person.bounds);
    assert_eq!(
      person.bounds_in(40, 10),
      PersonBounds {
        x: 10,
        y: 2,
        width: 22,
        height: 8
      }
    );
  }

  #[test]
  fn the_built_in_manifest_describes_instances() {
    let manifest = abra_ai_core::Manifest::parse(MANIFEST).unwrap();
    assert_eq!(manifest.decoder(), Some("instance-segmentation"));
    assert!(abra_ai_core::InputSpec::from_manifest(&manifest).is_ok());
    assert!(abra_ai_core::Decoder::from_manifest(&manifest).is_ok());
  }
}
