//! Turning a picture into the tensor a model expects.
//!
//! [`InputSpec`] says how: the size the model takes, whether the picture is stretched to it or fitted inside it with
//! padding, the order of the numbers (`nchw` or `nhwc`) and the normalization. It is read from the `input_*` params of a
//! model's [`Manifest`]. [`Placement`] remembers where the picture ended up inside the input, so a decoder can map
//! the model's answer back onto the original picture.

use crate::{error::AiError, manifest::Manifest};
use abra_core::{Image, ResizeTarget, Size, Transform, TransformAlgorithm};
use mask::Mask;

/// The order of the numbers in an input tensor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Layout {
  /// All the red values, then all the green, then all the blue: `[1, 3, height, width]`.
  Nchw,
  /// The three values of a pixel together: `[1, height, width, 3]`.
  Nhwc,
}

impl Layout {
  pub(crate) fn from_name(p_name: Option<&str>, p_default: Layout) -> Result<Self, AiError> {
    match p_name {
      None => Ok(p_default),
      Some("nchw") => Ok(Layout::Nchw),
      Some("nhwc") => Ok(Layout::Nhwc),
      Some(other) => Err(AiError::config_error(format!("Unknown tensor layout `{other}`; use nchw or nhwc"))),
    }
  }
}

/// How a picture of any shape is made to fit the model's input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fit {
  /// Stretch the picture to the input size.
  Stretch,
  /// Keep the picture's proportions, center it and fill the rest with a gray from `0` to `1`.
  Letterbox { pad: f32 },
}

/// How to build the input tensor for a model.
#[derive(Clone, Debug, PartialEq)]
pub struct InputSpec {
  pub width: u32,
  pub height: u32,
  pub layout: Layout,
  pub fit: Fit,
  /// Subtracted from each channel after scaling to `0..=1`.
  pub mean: [f32; 3],
  /// Each channel is divided by this after the mean is subtracted.
  pub standard_deviation: [f32; 3],
  /// Whether the model takes a guide, such as a trimap, as a fourth channel after the three colors. The guide is a [`Mask`]
  /// given with the picture, scaled to `0..=1` and not normalized. Where the picture is padded, the guide is `0`.
  pub guide: bool,
}

/// Where a picture sits inside the model's input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
  pub input_width: u32,
  pub input_height: u32,
  pub resized_width: u32,
  pub resized_height: u32,
  pub pad_x: u32,
  pub pad_y: u32,
  pub scale_x: f32,
  pub scale_y: f32,
}

/// The tensor for one picture and where the picture sits in it.
#[derive(Clone, Debug)]
pub struct Prepared {
  pub data: Vec<f32>,
  pub shape: Vec<usize>,
  pub placement: Placement,
}

impl Placement {
  /// A picture of `p_width` by `p_height` placed into an input of `p_input_width` by `p_input_height`.
  pub fn new(p_input_width: u32, p_input_height: u32, p_width: u32, p_height: u32, p_fit: Fit) -> Self {
    let (resized_width, resized_height) = match p_fit {
      Fit::Stretch => (p_input_width, p_input_height),
      Fit::Letterbox { .. } => {
        let scale = (p_input_width as f32 / p_width as f32).min(p_input_height as f32 / p_height as f32);
        (((p_width as f32 * scale).max(1.0) as u32), ((p_height as f32 * scale).max(1.0) as u32))
      }
    };
    Self {
      input_width: p_input_width,
      input_height: p_input_height,
      resized_width,
      resized_height,
      pad_x: (p_input_width - resized_width) / 2,
      pad_y: (p_input_height - resized_height) / 2,
      scale_x: resized_width as f32 / p_width as f32,
      scale_y: resized_height as f32 / p_height as f32,
    }
  }

  /// A mask at the size of the model's input, cut down to the part the picture covers and scaled to `p_width` by
  /// `p_height`: the model's answer in the picture's own pixels.
  pub fn restore(&self, p_input_mask: &Mask, p_width: u32, p_height: u32, p_algorithm: TransformAlgorithm) -> Mask {
    if (self.resized_width, self.resized_height) == (self.input_width, self.input_height) {
      return p_input_mask.resized(p_width, p_height, p_algorithm);
    }
    let values = p_input_mask.values();
    let mut cropped = Vec::with_capacity((self.resized_width * self.resized_height) as usize);
    for y in self.pad_y..self.pad_y + self.resized_height {
      let start = (y * self.input_width + self.pad_x) as usize;
      cropped.extend_from_slice(&values[start..start + self.resized_width as usize]);
    }
    Mask::from_values(self.resized_width, self.resized_height, cropped).resized(p_width, p_height, p_algorithm)
  }
}

impl InputSpec {
  /// Reads the input description from a manifest's params. `input_size` (or `input_width` and `input_height`) is
  /// required. The rest are `input_layout` (`nchw`, the default, or `nhwc`), `input_fit` (`stretch`, the default, or
  /// `letterbox`), `input_pad` (the letterbox gray from `0` to `1`, default `0.5`), `input_mean` and
  /// `input_standard_deviation` (three numbers each; default `0` and `1`), and `input_guide` (`1` when the model takes a
  /// guide mask as a fourth channel; default `0`).
  pub fn from_manifest(p_manifest: &Manifest) -> Result<Self, AiError> {
    let size = p_manifest.number("input_size");
    let dimension = |key: &str| {
      size.or_else(|| p_manifest.number(key)).filter(|value| *value >= 1.0).map(|value| value as u32).ok_or_else(|| {
        AiError::config_error(format!("The manifest of {} needs `input_size` in its params", p_manifest.name()))
      })
    };
    let triple = |key: &str, default: f32| -> Result<[f32; 3], AiError> {
      match p_manifest.list(key) {
        None => Ok([default; 3]),
        Some([a, b, c]) => Ok([*a as f32, *b as f32, *c as f32]),
        Some(_) => {
          Err(AiError::config_error(format!("`{key}` in the manifest of {} needs three numbers", p_manifest.name())))
        }
      }
    };
    let fit = match p_manifest.text("input_fit") {
      None | Some("stretch") => Fit::Stretch,
      Some("letterbox") => Fit::Letterbox {
        pad: p_manifest.number("input_pad").unwrap_or(0.5) as f32,
      },
      Some(other) => {
        return Err(AiError::config_error(format!("Unknown input fit `{other}`; use stretch or letterbox")));
      }
    };
    Ok(Self {
      width: dimension("input_width")?,
      height: dimension("input_height")?,
      layout: Layout::from_name(p_manifest.text("input_layout"), Layout::Nchw)?,
      fit,
      mean: triple("input_mean", 0.0)?,
      standard_deviation: triple("input_standard_deviation", 1.0)?,
      guide: p_manifest.number("input_guide").is_some_and(|value| value != 0.0),
    })
  }

  /// The number of channels the model takes: the three colors, and the guide when there is one.
  pub fn channels(&self) -> usize {
    3 + self.guide as usize
  }

  /// The tensor shape the model takes.
  pub fn shape(&self) -> Vec<usize> {
    match self.layout {
      Layout::Nchw => vec![1, self.channels(), self.height as usize, self.width as usize],
      Layout::Nhwc => vec![1, self.height as usize, self.width as usize, self.channels()],
    }
  }

  /// Builds the tensor for a picture.
  pub fn prepare(&self, p_image: &Image) -> Prepared {
    self.prepare_guided(p_image, None)
  }

  /// Builds the tensor for a picture and, when the model takes one, its guide. The guide is fitted to the model's input the
  /// same way as the picture, so it should have the picture's proportions.
  ///
  /// - `p_guide`: The guide mask, such as a trimap; it is scaled to the size of the picture. Ignored when the model takes no
  ///   guide, and read as all `0` when the model takes one and none is given.
  pub fn prepare_guided(&self, p_image: &Image, p_guide: Option<&Mask>) -> Prepared {
    let (width, height) = p_image.dimensions::<u32>();
    let placement = Placement::new(self.width, self.height, width, height, self.fit);
    let mut resized = p_image.clone();
    resized.resize(
      ResizeTarget::Exact(Size::new(placement.resized_width, placement.resized_height)),
      TransformAlgorithm::Bilinear,
    );
    let source = resized.rgba();
    let guide = self
      .guide
      .then(|| {
        p_guide
          .map(|guide| guide.resized(placement.resized_width, placement.resized_height, TransformAlgorithm::Bilinear))
      })
      .flatten();

    let normalize = |value: f32, channel: usize| (value - self.mean[channel]) / self.standard_deviation[channel];
    let plane = self.width as usize * self.height as usize;
    let channels = self.channels();
    let mut data = match self.fit {
      Fit::Stretch => vec![0.0; plane * channels],
      Fit::Letterbox { pad } => {
        let mut data = vec![0.0; plane * channels];
        for channel in 0..3 {
          let value = normalize(pad, channel);
          match self.layout {
            Layout::Nchw => data[channel * plane..(channel + 1) * plane].fill(value),
            Layout::Nhwc => data.iter_mut().skip(channel).step_by(channels).for_each(|slot| *slot = value),
          }
        }
        data
      }
    };
    for y in 0..placement.resized_height as usize {
      for x in 0..placement.resized_width as usize {
        let pixel = &source[(y * placement.resized_width as usize + x) * 4..];
        let index = (y + placement.pad_y as usize) * self.width as usize + x + placement.pad_x as usize;
        for channel in 0..3 {
          let value = normalize(pixel[channel] as f32 / 255.0, channel);
          match self.layout {
            Layout::Nchw => data[channel * plane + index] = value,
            Layout::Nhwc => data[index * channels + channel] = value,
          }
        }
        if let Some(guide) = &guide {
          let value = guide.values()[y * placement.resized_width as usize + x] as f32 / 255.0;
          match self.layout {
            Layout::Nchw => data[3 * plane + index] = value,
            Layout::Nhwc => data[index * channels + 3] = value,
          }
        }
      }
    }
    Prepared {
      data,
      shape: self.shape(),
      placement,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn spec(p_fit: Fit, p_layout: Layout) -> InputSpec {
    InputSpec {
      width: 4,
      height: 4,
      layout: p_layout,
      fit: p_fit,
      mean: [0.0; 3],
      standard_deviation: [1.0; 3],
      guide: false,
    }
  }

  #[test]
  fn letterbox_keeps_proportions_and_centers_the_padding() {
    let placement = Placement::new(640, 640, 1280, 640, Fit::Letterbox { pad: 0.5 });
    assert_eq!((placement.resized_width, placement.resized_height), (640, 320));
    assert_eq!((placement.pad_x, placement.pad_y), (0, 160));
  }

  #[test]
  fn stretch_fills_the_input() {
    let placement = Placement::new(256, 256, 100, 50, Fit::Stretch);
    assert_eq!((placement.resized_width, placement.resized_height, placement.pad_x, placement.pad_y), (256, 256, 0, 0));
  }

  #[test]
  fn the_same_picture_is_laid_out_both_ways() {
    let image = Image::new_from_color(2, 2, abra_core::Color::from_rgb(255, 0, 0));
    let chw = spec(Fit::Stretch, Layout::Nchw).prepare(&image);
    let hwc = spec(Fit::Stretch, Layout::Nhwc).prepare(&image);
    assert_eq!(chw.shape, [1, 3, 4, 4]);
    assert_eq!(hwc.shape, [1, 4, 4, 3]);
    assert!(chw.data[..16].iter().all(|v| (*v - 1.0).abs() < 1e-6), "the red plane comes first");
    assert!(chw.data[16..].iter().all(|v| *v == 0.0));
    assert_eq!(&hwc.data[..3], &[1.0, 0.0, 0.0], "a pixel's three values sit together");
  }

  #[test]
  fn letterbox_pads_with_the_gray_and_normalizes_it() {
    let mut input = spec(Fit::Letterbox { pad: 0.5 }, Layout::Nchw);
    input.mean = [0.5; 3];
    input.standard_deviation = [0.25; 3];
    let image = Image::new_from_color(4, 2, abra_core::Color::from_rgb(255, 255, 255));
    let prepared = input.prepare(&image);
    assert_eq!(prepared.data[0], 0.0, "the pad is the mean, so it normalizes to zero");
    assert!((prepared.data[prepared.placement.pad_y as usize * 4] - 2.0).abs() < 1e-5, "(1 - 0.5) / 0.25");
  }

  #[test]
  fn a_guide_is_the_fourth_channel_and_zero_in_the_padding() {
    let mut input = spec(Fit::Letterbox { pad: 0.5 }, Layout::Nchw);
    input.guide = true;
    let image = Image::new_from_color(4, 2, abra_core::Color::from_rgb(255, 255, 255));
    let guide = Mask::from_values(4, 2, vec![255; 8]);
    let prepared = input.prepare_guided(&image, Some(&guide));
    assert_eq!(prepared.shape, [1, 4, 4, 4]);
    let plane = &prepared.data[3 * 16..];
    assert_eq!(plane.iter().filter(|value| **value == 1.0).count(), 8, "the guide covers the picture");
    assert_eq!(plane.iter().filter(|value| **value == 0.0).count(), 8, "and is zero in the padding");
    assert_eq!(input.prepare(&image).data[3 * 16..], [0.0; 16], "no guide given is all zero");
    let mut hwc = input.clone();
    hwc.layout = Layout::Nhwc;
    assert_eq!(hwc.prepare_guided(&image, Some(&guide)).shape, [1, 4, 4, 4]);
  }

  #[test]
  fn restore_cuts_the_padding_and_scales_to_the_picture() {
    let placement = Placement::new(4, 4, 8, 4, Fit::Letterbox { pad: 0.0 });
    let mut values = vec![0u8; 16];
    values[4..12].fill(200);
    let restored = placement.restore(&Mask::from_values(4, 4, values), 8, 4, TransformAlgorithm::NearestNeighbor);
    assert_eq!((restored.width(), restored.height()), (8, 4));
    assert!(restored.values().iter().all(|value| *value == 200));
  }

  #[test]
  fn reads_the_input_params_from_a_manifest() {
    let manifest = Manifest::parse(
      "params:\n  input_size: 512\n  input_layout: nhwc\n  input_fit: letterbox\n  input_pad: 0.25\n  input_mean: [0.5, 0.5, 0.5]\n",
    )
    .unwrap();
    let input = InputSpec::from_manifest(&manifest).unwrap();
    assert_eq!((input.width, input.height, input.layout), (512, 512, Layout::Nhwc));
    assert_eq!(input.fit, Fit::Letterbox { pad: 0.25 });
    assert_eq!(input.mean, [0.5; 3]);
    assert!(InputSpec::from_manifest(&Manifest::parse("name: x\n").unwrap()).is_err());
    assert!(
      InputSpec::from_manifest(&Manifest::parse("params:\n  input_size: 8\n  input_layout: x\n").unwrap()).is_err()
    );
  }
}
