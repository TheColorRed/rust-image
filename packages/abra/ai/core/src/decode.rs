//! Reading a model's output.
//!
//! A [`Decoder`] knows one kind of model output and turns the raw tensors into something an editor can use. The manifest
//! names the decoder and gives its settings, so a new model of a known kind is two files, and a new kind of model is one
//! new decoder here.
//!
//! | `decoder` | The model returns | [`AiOutput`] |
//! |---|---|---|
//! | `semantic-mask` | a score for every class at every pixel | [`AiOutput::Mask`]: how likely the chosen classes are |
//! | `matte` | one value from 0 to 1 at every pixel (matting) | [`AiOutput::Mask`]: the model's alpha as it is |
//! | `label-map` | a score for every class at every pixel | [`AiOutput::Labels`]: the best class at every pixel |
//! | `instance-segmentation` | detections and mask prototypes (YOLO) | [`AiOutput::Instances`]: a box and mask for each |
//! | `landmarks` | points and a presence score | [`AiOutput::Landmarks`] |

use crate::{
  error::AiError,
  input::{Layout, Placement},
  manifest::Manifest,
};
use abra_core::{PointF, TransformAlgorithm};
use mask::Mask;

/// One output tensor: its shape and its values.
pub type Tensor = (Vec<usize>, Vec<f32>);

/// A box in the pixels of the picture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
  pub x: u32,
  pub y: u32,
  pub width: u32,
  pub height: u32,
}

/// One thing the model found: what it is, where, how sure the model is, and its soft silhouette.
#[derive(Clone, Debug)]
pub struct Instance {
  pub class: i32,
  pub bounds: Bounds,
  pub confidence: f32,
  pub mask: Mask,
}

/// The class of every pixel of a picture.
#[derive(Clone, Debug)]
pub struct LabelMap {
  pub width: u32,
  pub height: u32,
  /// One class number for each pixel, row by row from the top left.
  pub labels: Vec<u8>,
  /// The names of the classes, from the manifest. Empty when it lists none.
  pub classes: Vec<String>,
}

impl LabelMap {
  /// The class at a pixel, or nothing outside the picture.
  pub fn label_at(&self, p_x: u32, p_y: u32) -> Option<u8> {
    if p_x >= self.width || p_y >= self.height {
      return None;
    }
    Some(self.labels[p_y as usize * self.width as usize + p_x as usize])
  }

  /// A hard mask that is `255` on the pixels of one class and `0` elsewhere.
  pub fn mask(&self, p_class: u8) -> Mask {
    self.mask_of(&[p_class])
  }

  /// A hard mask that is `255` on the pixels of any of the classes and `0` elsewhere.
  /// - `p_classes`: The class numbers to select.
  pub fn mask_of(&self, p_classes: &[u8]) -> Mask {
    Mask::from_values(
      self.width,
      self.height,
      self.labels.iter().map(|label| if p_classes.contains(label) { 255 } else { 0 }).collect(),
    )
  }

  /// A hard mask that is `255` on the pixels of any of the named classes and `0` elsewhere. A name that is not one of
  /// the model's [`LabelMap::classes`] selects nothing.
  /// - `p_names`: The class names to select, as the manifest lists them.
  pub fn mask_named(&self, p_names: &[&str]) -> Mask {
    let classes: Vec<u8> = self
      .classes
      .iter()
      .enumerate()
      .filter(|(_, name)| p_names.contains(&name.as_str()))
      .map(|(index, _)| index as u8)
      .collect();
    self.mask_of(&classes)
  }
}

/// One point of a face or body, with a relative depth. Smaller depths are closer to the camera.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Landmark {
  pub point: PointF,
  pub depth: f32,
}

/// Points found in a picture and how sure the model is that the thing is there.
#[derive(Clone, Debug)]
pub struct Landmarks {
  pub landmarks: Vec<Landmark>,
  /// From `0` to `1`.
  pub presence: f32,
}

/// What a model found.
#[derive(Clone, Debug)]
pub enum AiOutput {
  Mask(Mask),
  Instances(Vec<Instance>),
  Labels(LabelMap),
  Landmarks(Landmarks),
}

/// Reads the class scores at each pixel of a model's output.
#[derive(Clone, Debug)]
pub struct ClassScores {
  class_count: usize,
  layout: Layout,
}

/// Softmax of the chosen classes at each pixel, as a mask.
#[derive(Clone, Debug)]
pub struct SemanticMask {
  scores: ClassScores,
  mask_classes: Vec<usize>,
}

/// One alpha value at each pixel, read as it is.
#[derive(Clone, Debug)]
pub struct Matte;

/// The best class at each pixel.
#[derive(Clone, Debug)]
pub struct LabelMapDecoder {
  scores: ClassScores,
  classes: Vec<String>,
}

/// YOLO-style instance segmentation: detections followed by shared mask prototypes.
#[derive(Clone, Debug)]
pub struct InstanceSegmentation {
  class: i32,
  confidence_threshold: f32,
  detection_count: usize,
  detection_size: usize,
  mask_channels: usize,
  edge_snap: f32,
}

/// Points, then one raw presence score.
#[derive(Clone, Debug)]
pub struct LandmarkDecoder {
  landmark_count: usize,
  coordinates_per_landmark: usize,
}

/// Turns a model's output into an [`AiOutput`].
#[derive(Clone, Debug)]
pub enum Decoder {
  SemanticMask(SemanticMask),
  Matte(Matte),
  LabelMap(LabelMapDecoder),
  InstanceSegmentation(InstanceSegmentation),
  Landmarks(LandmarkDecoder),
}

impl Decoder {
  /// The decoder a manifest names, with its settings (see each decoder's params below).
  ///
  /// - `semantic-mask`: `class_count`, `mask_classes` (the classes that count as the mask) and `output_layout`
  ///   (`nhwc` or `nchw`, default `nchw`).
  /// - `matte`: no params. The first output is `[1, 1, height, width]` at the size of the model's input, from `0` to `1`.
  /// - `label-map`: `class_count` and `output_layout`; the manifest's `classes` name them.
  /// - `instance-segmentation`: `class`, `confidence_threshold`, `detection_count`, `detection_size`, `mask_channels`
  ///   and `edge_snap`. A detection is `x1, y1, x2, y2, confidence, class, mask coefficients...`.
  /// - `landmarks`: `landmark_count` and `coordinates_per_landmark` (default `3`).
  pub fn from_manifest(p_manifest: &Manifest) -> Result<Self, AiError> {
    let scores = || -> Result<ClassScores, AiError> {
      Ok(ClassScores {
        class_count: p_manifest.required_number("class_count")? as usize,
        layout: Layout::from_name(p_manifest.text("output_layout"), Layout::Nchw)?,
      })
    };
    match p_manifest.decoder() {
      Some("semantic-mask") => {
        let mask_classes: Vec<usize> =
          p_manifest.list("mask_classes").unwrap_or_default().iter().map(|class| *class as usize).collect();
        let scores = scores()?;
        if mask_classes.is_empty() || mask_classes.iter().any(|class| *class >= scores.class_count) {
          return Err(AiError::config_error("`mask_classes` must name classes below `class_count`"));
        }
        Ok(Decoder::SemanticMask(SemanticMask { scores, mask_classes }))
      }
      Some("matte") => Ok(Decoder::Matte(Matte)),
      Some("label-map") => {
        let scores = scores()?;
        if scores.class_count == 0 || scores.class_count > 256 {
          return Err(AiError::config_error("`class_count` must be from 1 to 256 to label pixels with a byte"));
        }
        Ok(Decoder::LabelMap(LabelMapDecoder {
          scores,
          classes: p_manifest.classes().to_vec(),
        }))
      }
      Some("instance-segmentation") => Ok(Decoder::InstanceSegmentation(InstanceSegmentation {
        class: p_manifest.required_number("class")? as i32,
        confidence_threshold: p_manifest.required_number("confidence_threshold")? as f32,
        detection_count: p_manifest.required_number("detection_count")? as usize,
        detection_size: p_manifest.required_number("detection_size")? as usize,
        mask_channels: p_manifest.required_number("mask_channels")? as usize,
        edge_snap: p_manifest.number("edge_snap").unwrap_or(0.0) as f32,
      })),
      Some("landmarks") => Ok(Decoder::Landmarks(LandmarkDecoder {
        landmark_count: p_manifest.required_number("landmark_count")? as usize,
        coordinates_per_landmark: p_manifest.number("coordinates_per_landmark").unwrap_or(3.0) as usize,
      })),
      Some(other) => Err(AiError::config_error(format!(
        "Unknown decoder `{other}`; use semantic-mask, matte, label-map, instance-segmentation or landmarks"
      ))),
      None => Err(AiError::config_error(format!("The manifest of {} names no `decoder`", p_manifest.name()))),
    }
  }

  /// Reads the output tensors of a model run on a picture of `p_width` by `p_height`.
  ///
  /// - `p_outputs`: Every output of the model, in the model's order.
  /// - `p_placement`: Where the picture sat in the model's input.
  pub fn decode(
    &self, p_outputs: &[Tensor], p_width: u32, p_height: u32, p_placement: &Placement,
  ) -> Result<AiOutput, AiError> {
    match self {
      Decoder::SemanticMask(decoder) => decoder.decode(p_outputs, p_width, p_height, p_placement).map(AiOutput::Mask),
      Decoder::Matte(decoder) => decoder.decode(p_outputs, p_width, p_height, p_placement).map(AiOutput::Mask),
      Decoder::LabelMap(decoder) => decoder.decode(p_outputs, p_width, p_height, p_placement).map(AiOutput::Labels),
      Decoder::InstanceSegmentation(decoder) => {
        decoder.decode(p_outputs, p_width, p_height, p_placement).map(AiOutput::Instances)
      }
      Decoder::Landmarks(decoder) => decoder.decode(p_outputs, p_width, p_height, p_placement).map(AiOutput::Landmarks),
    }
  }
}

impl ClassScores {
  /// The scores of a model run, checked against the model's input size: one per class at every input pixel.
  fn check<'a>(&self, p_outputs: &'a [Tensor], p_placement: &Placement) -> Result<&'a [f32], AiError> {
    let (width, height) = (p_placement.input_width as usize, p_placement.input_height as usize);
    let (shape, values) =
      p_outputs.first().ok_or_else(|| AiError::inference_failed("The model returned no outputs"))?;
    let expected = match self.layout {
      Layout::Nchw => [1, self.class_count, height, width],
      Layout::Nhwc => [1, height, width, self.class_count],
    };
    if shape != &expected || values.len() != self.class_count * width * height {
      return Err(AiError::inference_failed(format!(
        "Unexpected class-score output shape {shape:?}, expected {expected:?}"
      )));
    }
    Ok(values)
  }

  /// The score of a class at a pixel.
  fn at(&self, p_values: &[f32], p_plane: usize, p_pixel: usize, p_class: usize) -> f32 {
    match self.layout {
      Layout::Nchw => p_values[p_class * p_plane + p_pixel],
      Layout::Nhwc => p_values[p_pixel * self.class_count + p_class],
    }
  }
}

impl SemanticMask {
  fn decode(
    &self, p_outputs: &[Tensor], p_width: u32, p_height: u32, p_placement: &Placement,
  ) -> Result<Mask, AiError> {
    let values = self.scores.check(p_outputs, p_placement)?;
    let plane = p_placement.input_width as usize * p_placement.input_height as usize;
    let mask = (0..plane)
      .map(|pixel| {
        let logits: Vec<f32> =
          (0..self.scores.class_count).map(|class| self.scores.at(values, plane, pixel, class)).collect();
        (class_confidence(&logits, &self.mask_classes) * 255.0).round().clamp(0.0, 255.0) as u8
      })
      .collect();
    let model_mask = Mask::from_values(p_placement.input_width, p_placement.input_height, mask);
    Ok(p_placement.restore(&model_mask, p_width, p_height, TransformAlgorithm::Bilinear))
  }
}

/// The share of the softmax of `p_logits` that belongs to the chosen classes.
fn class_confidence(p_logits: &[f32], p_classes: &[usize]) -> f32 {
  let max_logit = p_logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
  let exp_sum: f32 = p_logits.iter().map(|logit| (logit - max_logit).exp()).sum();
  let chosen: f32 = p_classes.iter().map(|class| (p_logits[*class] - max_logit).exp()).sum();
  chosen / exp_sum
}

impl Matte {
  fn decode(
    &self, p_outputs: &[Tensor], p_width: u32, p_height: u32, p_placement: &Placement,
  ) -> Result<Mask, AiError> {
    let (shape, values) =
      p_outputs.first().ok_or_else(|| AiError::inference_failed("The model returned no outputs"))?;
    let (width, height) = (p_placement.input_width as usize, p_placement.input_height as usize);
    if shape != &[1, 1, height, width] || values.len() != width * height {
      return Err(AiError::inference_failed(format!(
        "Unexpected matte output shape {shape:?}, expected [1, 1, {height}, {width}]"
      )));
    }
    let model_mask = Mask::from_values(
      p_placement.input_width,
      p_placement.input_height,
      values.iter().map(|value| (value * 255.0).round().clamp(0.0, 255.0) as u8).collect(),
    );
    Ok(p_placement.restore(&model_mask, p_width, p_height, TransformAlgorithm::Bilinear))
  }
}

impl LabelMapDecoder {
  fn decode(
    &self, p_outputs: &[Tensor], p_width: u32, p_height: u32, p_placement: &Placement,
  ) -> Result<LabelMap, AiError> {
    let values = self.scores.check(p_outputs, p_placement)?;
    let plane = p_placement.input_width as usize * p_placement.input_height as usize;
    let labels = (0..plane)
      .map(|pixel| {
        (0..self.scores.class_count)
          .max_by(|left, right| {
            self.scores.at(values, plane, pixel, *left).total_cmp(&self.scores.at(values, plane, pixel, *right))
          })
          .expect("A label map has classes") as u8
      })
      .collect();
    let model_labels = Mask::from_values(p_placement.input_width, p_placement.input_height, labels);
    let restored = p_placement.restore(&model_labels, p_width, p_height, TransformAlgorithm::NearestNeighbor);
    Ok(LabelMap {
      width: p_width,
      height: p_height,
      labels: restored.values().to_vec(),
      classes: self.classes.clone(),
    })
  }
}

impl InstanceSegmentation {
  fn decode(
    &self, p_outputs: &[Tensor], p_width: u32, p_height: u32, p_placement: &Placement,
  ) -> Result<Vec<Instance>, AiError> {
    if p_outputs.len() != 2 {
      return Err(AiError::inference_failed(format!("Expected 2 segmentation outputs, got {}", p_outputs.len())));
    }
    let (detections_shape, detections) = &p_outputs[0];
    let (prototypes_shape, prototypes) = &p_outputs[1];
    if detections_shape != &[1, self.detection_count, self.detection_size]
      || detections.len() != self.detection_count * self.detection_size
    {
      return Err(AiError::inference_failed(format!("Unexpected detections output shape: {detections_shape:?}")));
    }
    if prototypes_shape.len() != 4 || prototypes_shape[0] != 1 || prototypes_shape[1] != self.mask_channels {
      return Err(AiError::inference_failed(format!("Unexpected mask prototypes shape: {prototypes_shape:?}")));
    }
    let mask_height = prototypes_shape[2];
    let mask_width = prototypes_shape[3];
    if mask_height == 0 || mask_width == 0 || prototypes.len() != self.mask_channels * mask_height * mask_width {
      return Err(AiError::inference_failed(format!("Invalid mask prototype data: {prototypes_shape:?}")));
    }

    let (input_width, input_height) = (p_placement.input_width as f32, p_placement.input_height as f32);
    let mut found = Vec::new();
    for detection in detections.chunks_exact(self.detection_size) {
      let confidence = detection[4];
      if !confidence.is_finite() || confidence < self.confidence_threshold || detection[5].round() as i32 != self.class
      {
        continue;
      }
      let [x1, y1, x2, y2] = [detection[0], detection[1], detection[2], detection[3]];
      if ![x1, y1, x2, y2].iter().all(|value| value.is_finite()) {
        continue;
      }
      let x1 = x1.clamp(0.0, input_width);
      let y1 = y1.clamp(0.0, input_height);
      let x2 = x2.clamp(0.0, input_width);
      let y2 = y2.clamp(0.0, input_height);
      if x2 <= x1 || y2 <= y1 {
        continue;
      }
      // The picture sits inside the padded model input; a side near one of its edges is moved onto that edge.
      let snap = |value: f32, edge: f32, reach: f32| if (value - edge).abs() <= reach { edge } else { value };
      let (left, top) = (p_placement.pad_x as f32, p_placement.pad_y as f32);
      let (right, bottom) = (left + p_placement.resized_width as f32, top + p_placement.resized_height as f32);
      let (reach_x, reach_y) =
        (self.edge_snap * p_placement.resized_width as f32, self.edge_snap * p_placement.resized_height as f32);
      let (x1, x2) = (snap(x1, left, reach_x), snap(x2, right, reach_x));
      let (y1, y2) = (snap(y1, top, reach_y), snap(y2, bottom, reach_y));

      let Some(bounds) = source_bounds(x1, y1, x2, y2, p_width, p_height, p_placement) else {
        continue;
      };
      let mask = self.instance_mask(
        &detection[6..],
        prototypes,
        (mask_width, mask_height),
        (x1, y1, x2, y2),
        (p_width, p_height),
        p_placement,
      );
      found.push(Instance {
        class: self.class,
        bounds,
        confidence,
        mask,
      });
    }
    // Where bodies overlap (a baby held by a parent), a pixel belongs to the body whose mask is strongest there, so a
    // body's mask does not include the body in front of it.
    if found.len() > 1 {
      let strengths: Vec<&[u8]> = found.iter().map(|instance| instance.mask.values()).collect();
      let resolved: Vec<Mask> = (0..found.len())
        .map(|index| {
          let values = (0..strengths[index].len())
            .map(|pixel| {
              let strength = strengths[index][pixel];
              let outranked =
                strengths.iter().enumerate().any(|(other, values)| other != index && values[pixel] > strength);
              if outranked { 0 } else { strength }
            })
            .collect();
          Mask::from_values(p_width, p_height, values)
        })
        .collect();
      for (instance, mask) in found.iter_mut().zip(resolved) {
        instance.mask = mask;
      }
    }
    Ok(found)
  }

  /// The silhouette of one detection: its coefficients mixed through the prototypes, kept inside its box and brought
  /// back to the picture's pixels.
  fn instance_mask(
    &self, p_coefficients: &[f32], p_prototypes: &[f32], p_prototype_size: (usize, usize), p_box: (f32, f32, f32, f32),
    p_size: (u32, u32), p_placement: &Placement,
  ) -> Mask {
    let (mask_width, mask_height) = p_prototype_size;
    let plane = mask_width * mask_height;
    let mut prototype_values = Vec::with_capacity(plane);
    for index in 0..plane {
      let logit = (0..self.mask_channels)
        .map(|channel| p_coefficients[channel] * p_prototypes[channel * plane + index])
        .sum::<f32>();
      let probability = if logit >= 0.0 {
        1.0 / (1.0 + (-logit).exp())
      } else {
        let exp = logit.exp();
        exp / (1.0 + exp)
      };
      prototype_values.push((probability * 255.0).round() as u8);
    }

    let (input_width, input_height) = (p_placement.input_width, p_placement.input_height);
    let prototype = Mask::from_values(mask_width as u32, mask_height as u32, prototype_values).resized(
      input_width,
      input_height,
      TransformAlgorithm::Bilinear,
    );
    let mut model_values = prototype.values().to_vec();
    for y in 0..input_height {
      for x in 0..input_width {
        let inside_box = (x as f32 + 0.5) >= p_box.0
          && (x as f32 + 0.5) < p_box.2
          && (y as f32 + 0.5) >= p_box.1
          && (y as f32 + 0.5) < p_box.3;
        if !inside_box {
          model_values[(y * input_width + x) as usize] = 0;
        }
      }
    }
    let model_mask = Mask::from_values(input_width, input_height, model_values);
    p_placement.restore(&model_mask, p_size.0, p_size.1, TransformAlgorithm::Bilinear)
  }
}

fn source_bounds(
  p_x1: f32, p_y1: f32, p_x2: f32, p_y2: f32, p_width: u32, p_height: u32, p_placement: &Placement,
) -> Option<Bounds> {
  let x1 = ((p_x1 - p_placement.pad_x as f32) / p_placement.scale_x).floor().clamp(0.0, p_width as f32) as u32;
  let y1 = ((p_y1 - p_placement.pad_y as f32) / p_placement.scale_y).floor().clamp(0.0, p_height as f32) as u32;
  let x2 = ((p_x2 - p_placement.pad_x as f32) / p_placement.scale_x).ceil().clamp(0.0, p_width as f32) as u32;
  let y2 = ((p_y2 - p_placement.pad_y as f32) / p_placement.scale_y).ceil().clamp(0.0, p_height as f32) as u32;
  if x2 <= x1 || y2 <= y1 {
    return None;
  }
  Some(Bounds {
    x: x1,
    y: y1,
    width: x2 - x1,
    height: y2 - y1,
  })
}

impl LandmarkDecoder {
  fn decode(
    &self, p_outputs: &[Tensor], p_width: u32, p_height: u32, p_placement: &Placement,
  ) -> Result<Landmarks, AiError> {
    if p_outputs.len() < 2 {
      return Err(AiError::inference_failed(format!(
        "Expected landmark and presence outputs, got {}",
        p_outputs.len()
      )));
    }
    let (landmark_shape, landmark_values) = &p_outputs[0];
    let expected_values = self.landmark_count * self.coordinates_per_landmark;
    if landmark_shape != &[1, 1, 1, expected_values] || landmark_values.len() != expected_values {
      return Err(AiError::inference_failed(format!("Unexpected landmarks output shape: {landmark_shape:?}")));
    }
    let (presence_shape, presence_values) = &p_outputs[1];
    if presence_shape != &[1, 1, 1, 1] || presence_values.len() != 1 {
      return Err(AiError::inference_failed(format!("Unexpected presence output shape: {presence_shape:?}")));
    }

    let scale_x = p_width as f32 / p_placement.input_width as f32;
    let scale_y = p_height as f32 / p_placement.input_height as f32;
    let landmarks = landmark_values
      .chunks_exact(self.coordinates_per_landmark)
      .map(|coordinates| Landmark {
        point: PointF::new(coordinates[0] * scale_x, coordinates[1] * scale_y),
        depth: coordinates.get(2).map_or(0.0, |depth| depth * scale_x),
      })
      .collect();
    Ok(Landmarks {
      landmarks,
      presence: sigmoid(presence_values[0]),
    })
  }
}

fn sigmoid(p_value: f32) -> f32 {
  if p_value >= 0.0 {
    1.0 / (1.0 + (-p_value).exp())
  } else {
    let exponent = p_value.exp();
    exponent / (1.0 + exponent)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::input::Fit;

  fn manifest(p_text: &str) -> Manifest {
    Manifest::parse(p_text).unwrap()
  }

  #[test]
  fn combines_the_chosen_classes() {
    let confidence = class_confidence(&[0.0, 0.0, 2.0, 3.0, 0.0, 0.0], &[2, 3]);
    let expected = (2.0_f32.exp() + 3.0_f32.exp()) / (2.0_f32.exp() + 3.0_f32.exp() + 4.0);
    assert!((confidence - expected).abs() < 1e-6);
  }

  #[test]
  fn a_semantic_mask_reads_either_layout() {
    let decoder = Decoder::from_manifest(&manifest(
      "decoder: semantic-mask\nparams:\n  class_count: 2\n  mask_classes: [1]\n  output_layout: nhwc\n",
    ))
    .unwrap();
    let placement = Placement::new(1, 2, 1, 2, Fit::Stretch);
    // Two pixels: the first is class 0, the second class 1.
    let output = decoder.decode(&[(vec![1, 2, 1, 2], vec![9.0, 0.0, 0.0, 9.0])], 1, 2, &placement).unwrap();
    let AiOutput::Mask(mask) = output else { panic!("expected a mask") };
    assert_eq!(mask.values(), &[0, 255]);
  }

  #[test]
  fn a_wrong_output_shape_is_an_error() {
    let decoder =
      Decoder::from_manifest(&manifest("decoder: semantic-mask\nparams:\n  class_count: 2\n  mask_classes: [1]\n"))
        .unwrap();
    let placement = Placement::new(1, 1, 1, 1, Fit::Stretch);
    assert!(decoder.decode(&[], 1, 1, &placement).is_err());
    assert!(decoder.decode(&[(vec![1, 3, 1, 1], vec![0.0; 3])], 1, 1, &placement).is_err());
  }

  #[test]
  fn a_matte_is_read_as_it_is_and_cut_to_the_picture() {
    let decoder = Decoder::from_manifest(&manifest(
      "decoder: matte
",
    ))
    .unwrap();
    // A 2 wide, 4 tall picture in a 4 x 4 input: the padding is the top and bottom row.
    let placement = Placement::new(4, 4, 4, 2, Fit::Letterbox { pad: 0.0 });
    let mut alpha = vec![0.0; 16];
    alpha[4..12].fill(0.5);
    let AiOutput::Mask(mask) = decoder.decode(&[(vec![1, 1, 4, 4], alpha.clone())], 4, 2, &placement).unwrap() else {
      panic!("a matte is a mask");
    };
    assert_eq!((mask.width(), mask.height()), (4, 2));
    assert!(mask.values().iter().all(|value| *value == 128), "0.5 is 128 of 255");
    assert!(decoder.decode(&[(vec![1, 1, 2, 2], vec![0.0; 4])], 4, 2, &placement).is_err());
    assert!(decoder.decode(&[], 4, 2, &placement).is_err());
  }

  #[test]
  fn a_label_map_names_and_masks_a_class() {
    let decoder = Decoder::from_manifest(&manifest(
      "decoder: label-map\nparams:\n  class_count: 3\nclasses: [background, skin, nose]\n",
    ))
    .unwrap();
    let placement = Placement::new(2, 2, 2, 2, Fit::Stretch);
    let mut logits = vec![0.0; 3 * 4];
    logits[2 * 4..].fill(1.0);
    let AiOutput::Labels(labels) = decoder.decode(&[(vec![1, 3, 2, 2], logits)], 4, 4, &placement).unwrap() else {
      panic!("expected labels")
    };
    assert_eq!(labels.label_at(1, 1), Some(2));
    assert_eq!(labels.label_at(4, 0), None);
    assert_eq!(labels.classes[2], "nose");
    assert_eq!(labels.mask(2).values(), &[255; 16]);
    assert_eq!(labels.mask_named(&["skin", "nose"]).values(), &[255; 16], "any of the named classes");
    assert!(
      labels.mask_named(&["skin", "nothing"]).values().iter().all(|value| *value == 0),
      "a name that is not a class selects nothing"
    );
    assert!(labels.mask_of(&[0, 1]).values().iter().all(|value| *value == 0));
  }

  const INSTANCES: &str = "decoder: instance-segmentation\nparams:\n  class: 0\n  confidence_threshold: 0.25\n  detection_count: 300\n  detection_size: 38\n  mask_channels: 32\n  edge_snap: 0.04\n";

  fn instance_outputs(p_detections: Vec<f32>, p_prototypes: Vec<f32>) -> Vec<Tensor> {
    vec![(vec![1, 300, 38], p_detections), (vec![1, 32, 2, 2], p_prototypes)]
  }

  #[test]
  fn only_the_chosen_class_is_found_with_source_sized_masks() {
    let decoder = Decoder::from_manifest(&manifest(INSTANCES)).unwrap();
    let mut detections = vec![0.0; 300 * 38];
    detections[0..6].copy_from_slice(&[100.0, 100.0, 500.0, 500.0, 0.9, 0.0]);
    detections[6] = 10.0;
    detections[38..44].copy_from_slice(&[100.0, 100.0, 500.0, 500.0, 0.99, 2.0]);
    let mut prototypes = vec![0.0; 32 * 4];
    prototypes[..4].fill(1.0);
    let placement = Placement::new(640, 640, 64, 64, Fit::Letterbox { pad: 0.0 });
    let AiOutput::Instances(found) =
      decoder.decode(&instance_outputs(detections, prototypes), 64, 64, &placement).unwrap()
    else {
      panic!("expected instances")
    };
    assert_eq!(found.len(), 1);
    assert_eq!(
      found[0].bounds,
      Bounds {
        x: 10,
        y: 10,
        width: 40,
        height: 40
      }
    );
    assert_eq!((found[0].mask.width(), found[0].mask.height()), (64, 64));
    assert_eq!(found[0].mask.values()[0], 0, "pixels outside the box are excluded");
  }

  #[test]
  fn a_box_that_stops_just_short_of_the_picture_edge_is_moved_onto_it() {
    let decoder = Decoder::from_manifest(&manifest(INSTANCES)).unwrap();
    let mut detections = vec![0.0; 300 * 38];
    detections[0..6].copy_from_slice(&[100.0, 100.0, 520.0, 625.0, 0.9, 0.0]);
    let placement = Placement::new(640, 640, 64, 64, Fit::Letterbox { pad: 0.0 });
    let AiOutput::Instances(found) =
      decoder.decode(&instance_outputs(detections, vec![0.0; 32 * 4]), 64, 64, &placement).unwrap()
    else {
      panic!("expected instances")
    };
    assert_eq!(found[0].bounds.y + found[0].bounds.height, 64, "the bottom reaches the edge");
    assert_eq!(found[0].bounds.x + found[0].bounds.width, 52, "a side far from an edge is left alone");
  }

  #[test]
  fn an_overlapping_body_is_cut_out_of_the_body_behind_it() {
    let decoder = Decoder::from_manifest(&manifest(INSTANCES)).unwrap();
    let mut detections = vec![0.0; 300 * 38];
    detections[0..6].copy_from_slice(&[0.0, 0.0, 640.0, 640.0, 0.9, 0.0]);
    detections[6] = 1.0;
    detections[38..44].copy_from_slice(&[0.0, 0.0, 640.0, 640.0, 0.8, 0.0]);
    detections[38 + 7] = 1.0;
    let mut prototypes = vec![0.0; 32 * 4];
    prototypes[..4].fill(2.0);
    prototypes[4..8].copy_from_slice(&[0.0, 0.0, 6.0, 6.0]);
    let placement = Placement::new(640, 640, 64, 64, Fit::Letterbox { pad: 0.0 });
    let AiOutput::Instances(found) =
      decoder.decode(&instance_outputs(detections, prototypes), 64, 64, &placement).unwrap()
    else {
      panic!("expected instances")
    };
    let value = |instance: &Instance, y: u32| instance.mask.get(32, y).unwrap();
    assert!(value(&found[0], 4) > 128, "the first body keeps the top");
    assert_eq!(value(&found[0], 60), 0, "the second body is cut out of the first");
    assert!(value(&found[1], 60) > 128, "the second body keeps its own area");
    assert_eq!(value(&found[1], 4), 0, "the second body has no claim on the top");
  }

  #[test]
  fn landmarks_are_scaled_to_the_picture() {
    let decoder = Decoder::from_manifest(&manifest("decoder: landmarks\nparams:\n  landmark_count: 478\n")).unwrap();
    let mut values = vec![0.0; 478 * 3];
    values[..3].copy_from_slice(&[128.0, 64.0, -32.0]);
    let placement = Placement::new(256, 256, 512, 512, Fit::Stretch);
    let AiOutput::Landmarks(result) = decoder
      .decode(&[(vec![1, 1, 1, values.len()], values), (vec![1, 1, 1, 1], vec![0.0])], 512, 512, &placement)
      .unwrap()
    else {
      panic!("expected landmarks")
    };
    assert_eq!(result.landmarks.len(), 478);
    assert_eq!(
      result.landmarks[0],
      Landmark {
        point: PointF::new(256.0, 128.0),
        depth: -64.0
      }
    );
    assert_eq!(result.presence, 0.5);
  }

  #[test]
  fn an_unknown_or_missing_decoder_is_a_config_error() {
    assert!(Decoder::from_manifest(&manifest("decoder: nope\n")).is_err());
    assert!(Decoder::from_manifest(&manifest("name: x\n")).is_err());
    assert!(Decoder::from_manifest(&manifest("decoder: landmarks\n")).is_err());
    assert!(
      Decoder::from_manifest(&manifest("decoder: semantic-mask\nparams:\n  class_count: 2\n  mask_classes: [5]\n"))
        .is_err()
    );
  }
}
