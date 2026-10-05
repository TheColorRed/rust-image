//! YOLO26 instance segmentation for selecting people in an image.

use abra_ai_core::{AiError, onnx::OnnxSession};
use abra_core::{Channels, Image, ResizeTarget, Size, Transform, TransformAlgorithm};
pub use mask::Mask;
use std::sync::OnceLock;

const MODEL_BYTES: &[u8] = include_bytes!("../models/yolo26n-seg.onnx");
const INPUT_SIZE: u32 = 640;
const DETECTION_COUNT: usize = 300;
const DETECTION_SIZE: usize = 38;
const MASK_CHANNELS: usize = 32;
const PERSON_CLASS: i32 = 0;
const CONFIDENCE_THRESHOLD: f32 = 0.25;

/// The bounds of one detected person, in source-image pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PersonBounds {
  pub x: u32,
  pub y: u32,
  pub width: u32,
  pub height: u32,
}

/// One person instance and its soft, per-pixel silhouette mask.
#[derive(Clone, Debug)]
pub struct PersonInstance {
  pub bounds: PersonBounds,
  pub confidence: f32,
  pub mask: Mask,
}

/// Finds person instances using the bundled YOLO26n segmentation model.
pub struct PersonSegmenter {
  session: OnnxSession,
}

static PERSON_SEGMENTER: OnceLock<Result<PersonSegmenter, String>> = OnceLock::new();

/// Detects people and returns an individual silhouette mask for each one.
///
/// The model is loaded and cached on first use. The returned masks include the full person silhouette; intersect them
/// with a skin mask before applying skin-only effects.
pub fn find_people(p_image: &Image) -> Result<Vec<PersonInstance>, AiError> {
  let segmenter = PERSON_SEGMENTER
    .get_or_init(|| PersonSegmenter::load().map_err(|error| error.to_string()))
    .as_ref()
    .map_err(|error| AiError::model_load_failed(error.clone()))?;
  segmenter.process(p_image)
}

/// Multiplies a person silhouette mask by a skin-probability mask.
pub fn person_skin_mask(p_person: &Mask, p_skin: &Mask) -> Result<Mask, AiError> {
  let (width, height) = p_person.image().dimensions::<u32>();
  let (skin_width, skin_height) = p_skin.image().dimensions::<u32>();
  if width == 0 || height == 0 || skin_width == 0 || skin_height == 0 {
    return Err(AiError::invalid_input("Cannot combine empty masks"));
  }

  let mut skin_image = p_skin.image().clone();
  if (skin_width, skin_height) != (width, height) {
    skin_image.resize(ResizeTarget::Exact(Size::new(width, height)), TransformAlgorithm::Bilinear);
  }
  let pixels = p_person
    .image()
    .rgba()
    .chunks_exact(4)
    .zip(skin_image.rgba().chunks_exact(4))
    .flat_map(|(person, skin)| {
      let value = ((person[0] as u16 * skin[0] as u16 + 127) / 255) as u8;
      [value, value, value, 255]
    })
    .collect::<Vec<_>>();
  Ok(Mask::from_image(Image::new_from_pixels(width, height, pixels, Channels::RGBA)))
}

pub mod prelude {
  pub use crate::{PersonBounds, PersonInstance, PersonSegmenter, find_people, person_skin_mask};
  pub use abra_ai_core::AiError;
  pub use mask::Mask;
}

impl PersonSegmenter {
  /// Loads the bundled YOLO26n-seg ONNX model.
  pub fn load() -> Result<Self, AiError> {
    Ok(Self {
      session: OnnxSession::from_bytes(MODEL_BYTES, None)?,
    })
  }

  /// Finds people in `p_image`, returning source-sized instance masks.
  pub fn process(&self, p_image: &Image) -> Result<Vec<PersonInstance>, AiError> {
    let (width, height) = p_image.dimensions::<u32>();
    if width == 0 || height == 0 {
      return Err(AiError::invalid_input("Cannot find people in an empty image"));
    }

    let layout = Letterbox::new(width, height);
    let input = letterbox_input(p_image, layout);
    let outputs = self.session.run_multiple(&input, &[1, 3, INPUT_SIZE as usize, INPUT_SIZE as usize])?;
    decode_people(&outputs, width, height, layout)
  }
}

#[derive(Clone, Copy)]
struct Letterbox {
  resized_width: u32,
  resized_height: u32,
  pad_x: u32,
  pad_y: u32,
  scale_x: f32,
  scale_y: f32,
}

impl Letterbox {
  fn new(p_width: u32, p_height: u32) -> Self {
    let scale = (INPUT_SIZE as f32 / p_width as f32).min(INPUT_SIZE as f32 / p_height as f32);
    let resized_width = (p_width as f32 * scale).max(1.0) as u32;
    let resized_height = (p_height as f32 * scale).max(1.0) as u32;
    Self {
      resized_width,
      resized_height,
      pad_x: (INPUT_SIZE - resized_width) / 2,
      pad_y: (INPUT_SIZE - resized_height) / 2,
      scale_x: resized_width as f32 / p_width as f32,
      scale_y: resized_height as f32 / p_height as f32,
    }
  }
}

fn letterbox_input(p_image: &Image, p_layout: Letterbox) -> Vec<f32> {
  let mut resized = p_image.clone();
  resized.resize(
    ResizeTarget::Exact(Size::new(p_layout.resized_width, p_layout.resized_height)),
    TransformAlgorithm::Bilinear,
  );
  let source = resized.rgba();
  let plane_len = INPUT_SIZE as usize * INPUT_SIZE as usize;
  let mut input = vec![114.0 / 255.0; plane_len * 3];
  for y in 0..p_layout.resized_height as usize {
    for x in 0..p_layout.resized_width as usize {
      let source_index = (y * p_layout.resized_width as usize + x) * 4;
      let destination_index = (y + p_layout.pad_y as usize) * INPUT_SIZE as usize + x + p_layout.pad_x as usize;
      for channel in 0..3 {
        input[channel * plane_len + destination_index] = source[source_index + channel] as f32 / 255.0;
      }
    }
  }
  input
}

fn decode_people(
  p_outputs: &[(Vec<usize>, Vec<f32>)], p_width: u32, p_height: u32, p_layout: Letterbox,
) -> Result<Vec<PersonInstance>, AiError> {
  if p_outputs.len() != 2 {
    return Err(AiError::inference_failed(format!("Expected 2 YOLO26 segmentation outputs, got {}", p_outputs.len())));
  }

  let (detections_shape, detections) = &p_outputs[0];
  let (prototypes_shape, prototypes) = &p_outputs[1];
  if detections_shape != &[1, DETECTION_COUNT, DETECTION_SIZE] || detections.len() != DETECTION_COUNT * DETECTION_SIZE {
    return Err(AiError::inference_failed(format!("Unexpected YOLO26 detections output shape: {detections_shape:?}")));
  }
  if prototypes_shape.len() != 4 || prototypes_shape[0] != 1 || prototypes_shape[1] != MASK_CHANNELS {
    return Err(AiError::inference_failed(format!("Unexpected YOLO26 mask prototypes shape: {prototypes_shape:?}")));
  }
  let mask_height = prototypes_shape[2];
  let mask_width = prototypes_shape[3];
  if mask_height == 0 || mask_width == 0 || prototypes.len() != MASK_CHANNELS * mask_height * mask_width {
    return Err(AiError::inference_failed(format!("Invalid YOLO26 mask prototype data: {prototypes_shape:?}")));
  }

  let mut people = Vec::new();
  for detection in detections.chunks_exact(DETECTION_SIZE) {
    let confidence = detection[4];
    if !confidence.is_finite() || confidence < CONFIDENCE_THRESHOLD || detection[5].round() as i32 != PERSON_CLASS {
      continue;
    }

    let [x1, y1, x2, y2] = [detection[0], detection[1], detection[2], detection[3]];
    if ![x1, y1, x2, y2].iter().all(|value| value.is_finite()) {
      continue;
    }
    let x1 = x1.clamp(0.0, INPUT_SIZE as f32);
    let y1 = y1.clamp(0.0, INPUT_SIZE as f32);
    let x2 = x2.clamp(0.0, INPUT_SIZE as f32);
    let y2 = y2.clamp(0.0, INPUT_SIZE as f32);
    if x2 <= x1 || y2 <= y1 {
      continue;
    }

    let Some(bounds) = source_bounds(x1, y1, x2, y2, p_width, p_height, p_layout) else {
      continue;
    };
    let mask = instance_mask(
      &detection[6..],
      prototypes,
      mask_width,
      mask_height,
      (x1, y1, x2, y2),
      p_width,
      p_height,
      p_layout,
    );
    people.push(PersonInstance {
      bounds,
      confidence,
      mask,
    });
  }
  Ok(people)
}

fn source_bounds(
  p_x1: f32, p_y1: f32, p_x2: f32, p_y2: f32, p_width: u32, p_height: u32, p_layout: Letterbox,
) -> Option<PersonBounds> {
  let x1 = ((p_x1 - p_layout.pad_x as f32) / p_layout.scale_x).floor().clamp(0.0, p_width as f32) as u32;
  let y1 = ((p_y1 - p_layout.pad_y as f32) / p_layout.scale_y).floor().clamp(0.0, p_height as f32) as u32;
  let x2 = ((p_x2 - p_layout.pad_x as f32) / p_layout.scale_x).ceil().clamp(0.0, p_width as f32) as u32;
  let y2 = ((p_y2 - p_layout.pad_y as f32) / p_layout.scale_y).ceil().clamp(0.0, p_height as f32) as u32;
  if x2 <= x1 || y2 <= y1 {
    return None;
  }
  Some(PersonBounds {
    x: x1,
    y: y1,
    width: x2 - x1,
    height: y2 - y1,
  })
}

fn instance_mask(
  p_coefficients: &[f32], p_prototypes: &[f32], p_mask_width: usize, p_mask_height: usize, p_box: (f32, f32, f32, f32),
  p_width: u32, p_height: u32, p_layout: Letterbox,
) -> Mask {
  let mut prototype_pixels = Vec::with_capacity(p_mask_width * p_mask_height * 4);
  for y in 0..p_mask_height {
    for x in 0..p_mask_width {
      let index = y * p_mask_width + x;
      let logit = (0..MASK_CHANNELS)
        .map(|channel| p_coefficients[channel] * p_prototypes[channel * p_mask_width * p_mask_height + index])
        .sum::<f32>();
      let probability = if logit >= 0.0 {
        1.0 / (1.0 + (-logit).exp())
      } else {
        let exp = logit.exp();
        exp / (1.0 + exp)
      };
      let value = (probability * 255.0).round() as u8;
      prototype_pixels.extend([value, value, value, 255]);
    }
  }

  let mut prototype =
    Image::new_from_pixels(p_mask_width as u32, p_mask_height as u32, prototype_pixels, Channels::RGBA);
  prototype.resize(ResizeTarget::Exact(Size::new(INPUT_SIZE, INPUT_SIZE)), TransformAlgorithm::Bilinear);
  let model_pixels = prototype.rgba();
  let mut cropped_pixels = Vec::with_capacity((p_layout.resized_width * p_layout.resized_height * 4) as usize);
  for y in p_layout.pad_y..p_layout.pad_y + p_layout.resized_height {
    for x in p_layout.pad_x..p_layout.pad_x + p_layout.resized_width {
      let index = ((y * INPUT_SIZE + x) * 4) as usize;
      let inside_box = (x as f32 + 0.5) >= p_box.0
        && (x as f32 + 0.5) < p_box.2
        && (y as f32 + 0.5) >= p_box.1
        && (y as f32 + 0.5) < p_box.3;
      let value = if inside_box { model_pixels[index] } else { 0 };
      cropped_pixels.extend([value, value, value, 255]);
    }
  }
  let mut mask_image =
    Image::new_from_pixels(p_layout.resized_width, p_layout.resized_height, cropped_pixels, Channels::RGBA);
  mask_image.resize(ResizeTarget::Exact(Size::new(p_width, p_height)), TransformAlgorithm::Bilinear);
  Mask::from_image(mask_image)
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra_core::Color;

  #[test]
  fn letterbox_preserves_aspect_ratio_and_centers_padding() {
    let layout = Letterbox::new(1280, 640);
    assert_eq!((layout.resized_width, layout.resized_height), (640, 320));
    assert_eq!((layout.pad_x, layout.pad_y), (0, 160));
  }

  #[test]
  fn only_person_detections_receive_source_sized_masks() {
    let mut detections = vec![0.0; DETECTION_COUNT * DETECTION_SIZE];
    let person = &mut detections[..DETECTION_SIZE];
    person[0..6].copy_from_slice(&[100.0, 100.0, 500.0, 500.0, 0.9, PERSON_CLASS as f32]);
    person[6] = 10.0;
    let other_class = &mut detections[DETECTION_SIZE..DETECTION_SIZE * 2];
    other_class[0..6].copy_from_slice(&[100.0, 100.0, 500.0, 500.0, 0.99, 2.0]);

    let mut prototypes = vec![0.0; MASK_CHANNELS * 4];
    prototypes[..4].fill(1.0);
    let outputs = vec![
      (vec![1, DETECTION_COUNT, DETECTION_SIZE], detections),
      (vec![1, MASK_CHANNELS, 2, 2], prototypes),
    ];
    let layout = Letterbox::new(64, 64);
    let people = decode_people(&outputs, 64, 64, layout).unwrap();

    assert_eq!(people.len(), 1);
    assert_eq!(
      people[0].bounds,
      PersonBounds {
        x: 10,
        y: 10,
        width: 40,
        height: 40
      }
    );
    assert_eq!(people[0].mask.image().dimensions::<u32>(), (64, 64));
    assert_eq!(people[0].mask.image().rgba()[0], 0, "pixels outside the person box are excluded");
  }

  #[test]
  fn person_skin_mask_multiplies_soft_masks() {
    let person = Mask::from_image(Image::new_from_color(1, 1, Color::from_rgba(128, 128, 128, 255)));
    let skin = Mask::from_image(Image::new_from_color(1, 1, Color::from_rgba(128, 128, 128, 255)));
    let combined = person_skin_mask(&person, &skin).unwrap();
    assert_eq!(combined.image().rgba()[0], 64);
  }
}
