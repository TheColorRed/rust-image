//! A model described entirely by its manifest: what every input tensor is made of, and how the picture comes back.
//!
//! The single-picture path of [`AiModel`](crate::AiModel) feeds one image tensor and reads the answer with a named decoder. A model that
//! takes more than that (a picture and a mask, bytes instead of numbers, constants that start a state) is described by an `inputs` list in
//! its manifest instead, and nothing about it is written in Rust:
//!
//! ```yaml
//! decoder: image            # the model's answer is a picture
//! inputs:
//!   - name: image           # the tensor's name in the model
//!     from: image           # image, mask or constant
//!     type: f32             # f32 or u8
//!     size: 512             # omit to keep the picture's own size
//!   - name: mask
//!     from: mask            # the guide mask given with the picture
//!     type: f32
//!     size: 512
//!     on: 1                 # what the mask is where it covers the picture, and where it does not
//!     off: 0
//! window:                   # optional: run on a part of the picture round the mask
//!   margin: 0.5
//!   square: 1
//! params:
//!   composite: 1            # optional: only the pixels the mask covers change
//!   output_scale: 255       # what a pixel value of 255 is in the answer
//! ```
//!
//! An `image` input has `layout` (`nchw`, the default, or `nhwc`), `mean` and `standard_deviation` (three numbers each, applied to pixels
//! from `0` to `1`) and `scale` (what a pixel of `1` becomes afterwards). A `mask` input has the same `layout`. A `constant` input has
//! `value` and `shape`. For a `u8` input the numbers are the bytes of the pixels, and `on` and `off` are bytes.

use crate::{
  error::AiError,
  input::Layout,
  manifest::{Entry, Manifest},
  onnx::{AnyTensor, OnnxSession, TensorData},
};
use abra_core::{Channels, Image, Rect, ResizeTarget, Size, TransformAlgorithm, crop, resize};
use mask::Mask;

/// A mask value above this counts as covered, where a model needs a hard edge.
const COVERED: u8 = 127;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
  F32,
  U8,
}

#[derive(Clone, Debug, PartialEq)]
enum Source {
  Image,
  Mask,
  Constant { value: f32, shape: Vec<usize> },
}

/// One input tensor of the model.
#[derive(Clone, Debug, PartialEq)]
struct InputDef {
  name: String,
  source: Source,
  kind: Kind,
  layout: Layout,
  /// The side of the input, or `None` to keep the size of the part of the picture the model is given.
  size: Option<u32>,
  mean: [f32; 3],
  standard_deviation: [f32; 3],
  scale: f32,
  on: f32,
  off: f32,
}

/// Running on a part of the picture round the mask, and not on the whole picture.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Window {
  /// How much more to take on every side, as a share of the larger side of the mask.
  margin: f32,
  /// Whether the part is made square (kept inside the picture).
  square: bool,
}

/// A model that is run from its manifest's `inputs`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Graph {
  inputs: Vec<InputDef>,
  window: Option<Window>,
  composite: bool,
  output_layout: Layout,
  output_scale: f32,
  output_name: Option<String>,
}

fn kind(p_entry: &Entry) -> Result<Kind, AiError> {
  match p_entry.text("type") {
    None | Some("f32") => Ok(Kind::F32),
    Some("u8") => Ok(Kind::U8),
    Some(other) => Err(AiError::config_error(format!("Unknown input type `{other}`; use f32 or u8"))),
  }
}

fn triple(p_entry: &Entry, p_key: &str, p_default: f32) -> Result<[f32; 3], AiError> {
  match p_entry.list(p_key) {
    None => Ok([p_default; 3]),
    Some([a, b, c]) => Ok([*a as f32, *b as f32, *c as f32]),
    Some(_) => Err(AiError::config_error(format!("`{p_key}` of an input needs three numbers"))),
  }
}

impl InputDef {
  fn from_entry(p_entry: &Entry) -> Result<Self, AiError> {
    let name = p_entry.text("name").ok_or_else(|| AiError::config_error("An input of the manifest has no `name`"))?;
    let source = match p_entry.text("from") {
      Some("image") => Source::Image,
      Some("mask") => Source::Mask,
      Some("constant") => Source::Constant {
        value: p_entry.number("value").unwrap_or(0.0) as f32,
        shape: p_entry
          .list("shape")
          .map(|shape| shape.iter().map(|dimension| (*dimension).max(1.0) as usize).collect())
          .unwrap_or_else(|| vec![1]),
      },
      other => {
        return Err(AiError::config_error(format!(
          "The input `{name}` is from `{}`; use image, mask or constant",
          other.unwrap_or("(nothing)")
        )));
      }
    };
    Ok(Self {
      name: name.to_string(),
      source,
      kind: kind(p_entry)?,
      layout: Layout::from_name(p_entry.text("layout"), Layout::Nchw)?,
      size: p_entry.number("size").filter(|size| *size >= 1.0).map(|size| size as u32),
      mean: triple(p_entry, "mean", 0.0)?,
      standard_deviation: triple(p_entry, "standard_deviation", 1.0)?,
      scale: p_entry.number("scale").unwrap_or(1.0) as f32,
      on: p_entry.number("on").unwrap_or(1.0) as f32,
      off: p_entry.number("off").unwrap_or(0.0) as f32,
    })
  }

  /// The tensor for a part of the picture.
  fn image_tensor(&self, p_part: &Image) -> AnyTensor {
    let (width, height) = p_part.dimensions::<u32>();
    let side = self.size.unwrap_or(0);
    let (width, height) = if side == 0 { (width, height) } else { (side, side) };
    let scaled = resize(ResizeTarget::Exact(Size::new(width, height))).resized(p_part);
    let plane = width as usize * height as usize;
    let (index, shape): (Box<dyn Fn(usize, usize) -> usize>, Vec<usize>) = match self.layout {
      Layout::Nchw => (Box::new(move |channel, pixel| channel * plane + pixel), vec![1, 3, height as usize, width as usize]),
      Layout::Nhwc => (Box::new(|channel, pixel| pixel * 3 + channel), vec![1, height as usize, width as usize, 3]),
    };
    let data = match self.kind {
      Kind::U8 => {
        let mut data = vec![0u8; plane * 3];
        for (pixel, rgba) in scaled.rgba().chunks_exact(4).enumerate() {
          for channel in 0..3 {
            data[index(channel, pixel)] = rgba[channel];
          }
        }
        TensorData::U8(data)
      }
      Kind::F32 => {
        let mut data = vec![0f32; plane * 3];
        for (pixel, rgba) in scaled.rgba().chunks_exact(4).enumerate() {
          for channel in 0..3 {
            data[index(channel, pixel)] =
              (rgba[channel] as f32 / 255.0 - self.mean[channel]) / self.standard_deviation[channel] * self.scale;
          }
        }
        TensorData::F32(data)
      }
    };
    AnyTensor { shape, data }
  }

  /// The tensor for the mask over a part of the picture: `on` where it covers, `off` where it does not.
  fn mask_tensor(&self, p_part: &Mask) -> AnyTensor {
    let (width, height) = p_part.dimensions::<u32>();
    let side = self.size.unwrap_or(0);
    let scaled = if side == 0 || (side, side) == (width, height) {
      p_part.clone()
    } else {
      p_part.resized(side, side, TransformAlgorithm::Bilinear)
    };
    let (width, height) = scaled.dimensions::<usize>();
    let shape = match self.layout {
      Layout::Nchw => vec![1, 1, height, width],
      Layout::Nhwc => vec![1, height, width, 1],
    };
    let covered = |value: u8| if value > COVERED { self.on } else { self.off };
    let data = match self.kind {
      Kind::U8 => TensorData::U8(scaled.values().iter().map(|&value| covered(value) as u8).collect()),
      Kind::F32 => TensorData::F32(scaled.values().iter().map(|&value| covered(value)).collect()),
    };
    AnyTensor { shape, data }
  }
}

impl Graph {
  /// Reads the graph from a manifest, or `None` when the manifest has no `inputs` and so describes a model with a single picture input.
  pub(crate) fn from_manifest(p_manifest: &Manifest) -> Result<Option<Self>, AiError> {
    let entries = p_manifest.section("inputs");
    if entries.is_empty() {
      return Ok(None);
    }
    if p_manifest.decoder() != Some("image") {
      return Err(AiError::config_error(format!(
        "The manifest of {} lists `inputs`, which can only be read with `decoder: image`",
        p_manifest.name()
      )));
    }
    let window = p_manifest.section("window").first().map(|entry| Window {
      margin: entry.number("margin").unwrap_or(0.0).max(0.0) as f32,
      square: entry.number("square").is_some_and(|value| value != 0.0),
    });
    Ok(Some(Self {
      inputs: entries.iter().map(InputDef::from_entry).collect::<Result<_, _>>()?,
      window,
      composite: p_manifest.number("composite").is_some_and(|value| value != 0.0),
      output_layout: Layout::from_name(p_manifest.text("output_layout"), Layout::Nchw)?,
      output_scale: p_manifest.number("output_scale").unwrap_or(255.0) as f32,
      output_name: p_manifest.text("output_name").map(str::to_string),
    }))
  }

  /// Whether the model needs a guide mask given with the picture.
  pub(crate) fn needs_guide(&self) -> bool {
    self.inputs.iter().any(|input| input.source == Source::Mask) || self.window.is_some() || self.composite
  }

  /// Runs the model on a picture, and a guide mask when the model needs one, and gives the picture it answers with.
  pub(crate) fn run(&self, p_session: &OnnxSession, p_image: &Image, p_guide: Option<&Mask>) -> Result<Image, AiError> {
    let (width, height) = p_image.dimensions::<u32>();
    if let Some(guide) = p_guide {
      if guide.dimensions::<u32>() != (width, height) {
        return Err(AiError::invalid_input("The guide mask must be the size of the picture"));
      }
    }

    // The part of the picture the model is given: all of it, or the part round the mask.
    let whole = Rect::new((0, 0), (width, height));
    let part_rect = match (&self.window, p_guide) {
      (Some(window), Some(guide)) => match window_round(guide, (width, height), *window) {
        Some(rect) => rect,
        // Nothing is covered, so nothing changes.
        None => return Ok(p_image.clone()),
      },
      _ => whole,
    };
    let (left, top, right, bottom) = part_rect.edges::<u32>();
    let (part_width, part_height) = (right - left, bottom - top);
    let mut part = p_image.clone();
    crop(left, top, part_width, part_height).apply(&mut part);
    let part_guide = p_guide.map(|guide| guide.cropped(part_rect));

    let mut tensors = Vec::with_capacity(self.inputs.len());
    for input in &self.inputs {
      tensors.push(match &input.source {
        Source::Image => input.image_tensor(&part),
        Source::Mask => input.mask_tensor(
          part_guide.as_ref().ok_or_else(|| AiError::invalid_input("This model needs a guide mask with the picture"))?,
        ),
        Source::Constant { value, shape } => {
          let count = shape.iter().product();
          AnyTensor {
            shape: shape.clone(),
            data: match input.kind {
              Kind::F32 => TensorData::F32(vec![*value; count]),
              Kind::U8 => TensorData::U8(vec![*value as u8; count]),
            },
          }
        }
      });
    }
    let named: Vec<(&str, &AnyTensor)> =
      self.inputs.iter().map(|input| input.name.as_str()).zip(tensors.iter()).collect();
    let outputs = p_session.run_tensors(&named)?;
    let answer = match &self.output_name {
      Some(name) => outputs.iter().find(|(output, _)| output == name).map(|(_, tensor)| tensor),
      None => outputs.first().map(|(_, tensor)| tensor),
    }
    .ok_or_else(|| AiError::inference_failed("The model returned no picture"))?;
    let mut answered = self.picture_of(answer)?;

    // The answer goes back to the size of the part it came from when it is put back into the picture.
    if (self.composite || self.window.is_some()) && answered.dimensions::<u32>() != (part_width, part_height) {
      answered = resize(ResizeTarget::Exact(Size::new(part_width, part_height))).resized(&answered);
    }
    if !self.composite && self.window.is_none() {
      return Ok(answered);
    }

    // Put the part back. With `composite`, the guide's own values say how much of the answer each pixel takes, so a soft edge stays soft.
    let mut out = p_image.to_rgba_vec();
    for row in 0..part_height as usize {
      for column in 0..part_width as usize {
        let alpha = match (&part_guide, self.composite) {
          (Some(guide), true) => guide.values()[row * part_width as usize + column] as u32,
          _ => 255,
        };
        if alpha == 0 {
          continue;
        }
        let at = ((top as usize + row) * width as usize + left as usize + column) * 4;
        let from = (row * part_width as usize + column) * 4;
        for channel in 0..3 {
          out[at + channel] =
            ((out[at + channel] as u32 * (255 - alpha) + answered.rgba()[from + channel] as u32 * alpha + 127) / 255) as u8;
        }
      }
    }
    Ok(Image::new_from_pixels(width, height, out, Channels::RGBA))
  }

  /// The picture a tensor holds: three colors, in the order `output_layout` says, from `0` to `output_scale`.
  fn picture_of(&self, p_tensor: &AnyTensor) -> Result<Image, AiError> {
    let (height, width) = match (self.output_layout, p_tensor.shape.as_slice()) {
      (Layout::Nchw, [1, 3, height, width]) | (Layout::Nhwc, [1, height, width, 3]) => (*height, *width),
      _ => {
        return Err(AiError::inference_failed(format!("Unexpected picture shape {:?}", p_tensor.shape)));
      }
    };
    let plane = width * height;
    let value = |channel: usize, pixel: usize| -> u8 {
      let at = match self.output_layout {
        Layout::Nchw => channel * plane + pixel,
        Layout::Nhwc => pixel * 3 + channel,
      };
      let number = match &p_tensor.data {
        TensorData::F32(data) => data[at],
        TensorData::U8(data) => data[at] as f32,
      };
      (number / self.output_scale * 255.0).round().clamp(0.0, 255.0) as u8
    };
    let mut pixels = Vec::with_capacity(plane * 4);
    for pixel in 0..plane {
      pixels.extend_from_slice(&[value(0, pixel), value(1, pixel), value(2, pixel), 255]);
    }
    Ok(Image::new_from_pixels(width as u32, height as u32, pixels, Channels::RGBA))
  }
}

/// The part of the picture to give the model: the mask's bounds with `margin` more on every side, kept inside the picture, and made
/// square when the window says so. When the mask is larger than the picture's shorter side the part is not square. `None` when the mask
/// covers nothing.
fn window_round(p_mask: &Mask, p_picture: (u32, u32), p_window: Window) -> Option<Rect> {
  let (picture_width, picture_height) = p_picture;
  let (left, top, right, bottom) = p_mask.bounds(COVERED)?.edges::<f32>();
  let (mask_width, mask_height) = (right - left, bottom - top);
  let larger = mask_width.max(mask_height);
  let side = (larger * (1.0 + 2.0 * p_window.margin)).max(1.0);
  let (wanted_width, wanted_height) = if p_window.square {
    (side.max(mask_width), side.max(mask_height))
  } else {
    (mask_width * (1.0 + 2.0 * p_window.margin), mask_height * (1.0 + 2.0 * p_window.margin))
  };
  let width = wanted_width.min(picture_width as f32).ceil().max(1.0) as u32;
  let height = wanted_height.min(picture_height as f32).ceil().max(1.0) as u32;
  let x = (((left + right) / 2.0 - width as f32 / 2.0).round().max(0.0) as u32).min(picture_width - width);
  let y = (((top + bottom) / 2.0 - height as f32 / 2.0).round().max(0.0) as u32).min(picture_height - height);
  Some(Rect::new((x, y), (width, height)))
}

#[cfg(test)]
mod tests {
  use super::*;

  const MANIFEST: &str = "name: m\ndecoder: image\ninputs:\n  - name: image\n    from: image\n    type: u8\n    size: 8\n  - name: mask\n    from: mask\n    type: f32\n    size: 8\n    on: 1\n    off: 0\n  - name: state\n    from: constant\n    shape: [1, 2]\n    value: 0.5\nwindow:\n  margin: 0.5\n  square: 1\nparams:\n  composite: 1\n  output_scale: 255\n";

  #[test]
  fn the_inputs_window_and_composite_come_from_the_manifest() {
    let graph = Graph::from_manifest(&Manifest::parse(MANIFEST).unwrap()).unwrap().unwrap();
    assert_eq!(graph.inputs.len(), 3);
    assert_eq!((graph.inputs[0].kind, graph.inputs[0].size), (Kind::U8, Some(8)));
    assert_eq!((graph.inputs[1].on, graph.inputs[1].off), (1.0, 0.0));
    assert_eq!(graph.inputs[2].source, Source::Constant { value: 0.5, shape: vec![1, 2] });
    assert_eq!(graph.window, Some(Window { margin: 0.5, square: true }));
    assert!(graph.composite && graph.needs_guide());
  }

  #[test]
  fn a_manifest_without_inputs_is_a_single_picture_model_and_inputs_need_the_image_decoder() {
    assert!(Graph::from_manifest(&Manifest::parse("name: x\n").unwrap()).unwrap().is_none());
    assert!(Graph::from_manifest(&Manifest::parse("name: x\ndecoder: matte\ninputs:\n  - name: a\n    from: image\n").unwrap()).is_err());
    assert!(
      Graph::from_manifest(&Manifest::parse("name: x\ndecoder: image\ninputs:\n  - name: a\n    from: nowhere\n").unwrap()).is_err()
    );
  }

  #[test]
  fn the_window_is_a_square_round_the_mask_kept_inside_the_picture() {
    let mut values = vec![0u8; 400 * 300];
    for y in 10..50 {
      for x in 10..50 {
        values[y * 400 + x] = 255;
      }
    }
    let mask = Mask::from_values(400, 300, values);
    let window = window_round(&mask, (400, 300), Window { margin: 0.5, square: true }).unwrap();
    // 40 wide with half of that more on each side is 80, pushed back inside the picture at the top left.
    assert_eq!(window.edges::<u32>(), (0, 0, 80, 80));
    assert!(window_round(&Mask::empty(400, 300), (400, 300), Window { margin: 0.5, square: true }).is_none());
  }

  #[test]
  fn an_image_input_is_scaled_and_laid_out_as_the_manifest_says() {
    let entry = |text: &str| Manifest::parse(text).unwrap().section("inputs")[0].clone();
    let photo = Image::new_from_pixels(2, 1, vec![255, 0, 0, 255, 0, 0, 255, 255], Channels::RGBA);
    let nchw = InputDef::from_entry(&entry("inputs:\n  - name: a\n    from: image\n    size: 2\n")).unwrap().image_tensor(&photo);
    assert_eq!(nchw.shape, vec![1, 3, 2, 2]);
    let bytes = InputDef::from_entry(&entry("inputs:\n  - name: a\n    from: image\n    type: u8\n    layout: nhwc\n")).unwrap().image_tensor(&photo);
    assert_eq!((bytes.shape, bytes.data), (vec![1, 1, 2, 3], TensorData::U8(vec![255, 0, 0, 0, 0, 255])));
  }
}
