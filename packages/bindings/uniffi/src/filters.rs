//! Effects from `filters`: blurs, distortions, edges, noise, sharpening, smoothing and the Sobel filter.
//!
//! An effect is made by its function and then applied to an image, as in the library: `gaussian_blur(12).apply(image)`. The
//! functions every effect has from the library's `Effect` trait are written for each handle by [`effect_methods!`]. The
//! `with_*` functions return a new effect, as the library's builders take and return the effect by value.
//!
//! Not exported: `heal` (it borrows a `&dyn CoverageMask`, a trait object with a lifetime) and the `apply_kernel` helper (it
//! is private to the library).

use std::sync::Arc;

use abra::abra_core::Image as CoreImage;
use abra::filters::prelude::{
  blur, distort, edges, morphology, noise, sharpen as sharpen_module, smooth as smooth_module, sobel as sobel_module,
};
// The enums are redeclared below for uniffi, which needs the library's own type in scope under its own name.
use blur::{ApertureShape, FocusShape};
use distort::{RippleShape, RippleSize};
use noise::NoiseDistribution;
use sobel_module::SobelDirection;

use crate::{Image, Mask};
use crate::effect::effect_methods;

// The shape of the focus area of a focus blur.
#[uniffi::remote(Enum)]
pub enum FocusShape {
  Circle,
  Square,
  Diamond,
  Horizontal,
  Vertical,
}

// The aperture shape of a lens blur, by its number of blades.
#[uniffi::remote(Enum)]
pub enum ApertureShape {
  Triangle,
  Square,
  Pentagon,
  Hexagon,
  Heptagon,
  Octagon,
}

// How noise is spread: evenly, or in a normal distribution.
#[uniffi::remote(Enum)]
pub enum NoiseDistribution {
  Uniform,
  Gaussian,
}

// The direction of the Sobel kernel.
#[uniffi::remote(Enum)]
pub enum SobelDirection {
  Horizontal,
  Vertical,
}

// The size of the ripples of a ripple.
#[uniffi::remote(Enum)]
pub enum RippleSize {
  Small,
  Medium,
  Large,
}

// The shape of the ripples of a ripple.
#[uniffi::remote(Enum)]
pub enum RippleShape {
  Circular,
  Square,
  Angle(f32),
  Random,
}

/// A blur with no settings of its own. Made by [`blur`].
#[derive(uniffi::Object)]
pub struct Blur(blur::Blur);

/// A basic blur.
#[uniffi::export]
pub fn blur() -> Arc<Blur> {
  Arc::new(Blur(blur::blur()))
}

/// A box blur. Made by [`box_blur`].
#[derive(uniffi::Object)]
pub struct BoxBlur(blur::BoxBlur);

/// A box blur with this radius in pixels.
#[uniffi::export]
pub fn box_blur(radius: f64) -> Arc<BoxBlur> {
  Arc::new(BoxBlur(blur::box_blur(radius)))
}

/// A gaussian blur. Made by [`gaussian_blur`].
#[derive(uniffi::Object)]
pub struct GaussianBlur(blur::GaussianBlur);

/// A gaussian blur with this radius in pixels.
#[uniffi::export]
pub fn gaussian_blur(radius: f64) -> Arc<GaussianBlur> {
  Arc::new(GaussianBlur(blur::gaussian_blur(radius)))
}

/// A lens blur, where the aperture shape gives out-of-focus highlights their shape. Made by [`lens_blur`].
#[derive(uniffi::Object)]
pub struct LensBlur(blur::LensBlur);

/// A lens blur with this radius in pixels. By default the aperture is a hexagon with half-rounded blades, sampled 32 times per
/// pixel, with no specular boost or noise.
#[uniffi::export]
pub fn lens_blur(radius: f64) -> Arc<LensBlur> {
  Arc::new(LensBlur(blur::lens_blur(radius)))
}

#[uniffi::export]
impl LensBlur {
  /// The aperture shape, which shapes the bokeh. Defaults to a hexagon.
  pub fn with_shape(&self, shape: ApertureShape) -> Arc<LensBlur> {
    Arc::new(LensBlur(self.0.clone().with_shape(shape)))
  }

  /// How round the aperture blades are, from 0 (straight polygon) to 1 (circle). Defaults to 0.5.
  pub fn with_blade_curvature(&self, curvature: f32) -> Arc<LensBlur> {
    Arc::new(LensBlur(self.0.clone().with_blade_curvature(curvature)))
  }

  /// The clockwise rotation of the aperture in degrees. Defaults to 0.
  pub fn with_rotation(&self, degrees: f32) -> Arc<LensBlur> {
    Arc::new(LensBlur(self.0.clone().with_rotation(degrees)))
  }

  /// Brightens highlights to make bokeh stand out: samples brighter than `threshold` (0 to 1) are multiplied by `brightness`
  /// (at least 1). Off by default.
  pub fn with_specular(&self, brightness: f32, threshold: f32) -> Arc<LensBlur> {
    Arc::new(LensBlur(self.0.clone().with_specular(brightness, threshold)))
  }

  /// Adds noise after blurring, to match the grain of the rest of the image. `amount` is 0 to 1. Off by default.
  pub fn with_noise(&self, amount: f32, distribution: NoiseDistribution) -> Arc<LensBlur> {
    Arc::new(LensBlur(self.0.clone().with_noise(amount, distribution)))
  }

  /// The number of samples per pixel. Higher is smoother but slower. Defaults to 32.
  pub fn with_samples(&self, samples: u32) -> Arc<LensBlur> {
    Arc::new(LensBlur(self.0.clone().with_samples(samples)))
  }
}

/// A motion blur. Made by [`motion_blur`].
#[derive(uniffi::Object)]
pub struct MotionBlur(blur::MotionBlur);

/// A motion blur in the direction of `angle_degrees`, over `distance` pixels.
#[uniffi::export]
pub fn motion_blur(angle_degrees: f32, distance: u32) -> Arc<MotionBlur> {
  Arc::new(MotionBlur(blur::motion_blur(angle_degrees, distance)))
}

/// A blur that keeps edges. Made by [`surface_blur`].
#[derive(uniffi::Object)]
pub struct SurfaceBlur(blur::SurfaceBlur);

/// A surface blur with this radius in pixels, which leaves pixels that differ by more than `threshold` alone.
#[uniffi::export]
pub fn surface_blur(radius: u32, threshold: u8) -> Arc<SurfaceBlur> {
  Arc::new(SurfaceBlur(blur::surface_blur(radius, threshold)))
}

#[uniffi::export]
impl SurfaceBlur {
  /// The step between the pixels that are sampled.
  pub fn with_step(&self, step: u32) -> Arc<SurfaceBlur> {
    Arc::new(SurfaceBlur(self.0.clone().with_step(step as usize)))
  }
}

/// Where the focus area of a focus blur sits and how its edge fades into the blur. Distances are normalized so that 1 is half of
/// the image's shorter side.
#[derive(uniffi::Object)]
pub struct FocusGeometry(blur::FocusGeometry);

#[uniffi::export]
impl FocusGeometry {
  /// The default geometry: a circle in the center.
  #[uniffi::constructor]
  pub fn new() -> Self {
    Self(blur::FocusGeometry::new())
  }

  /// Horizontal center of the focus area as a fraction of the image width. Defaults to 0.5.
  pub fn with_center_x(&self, value: f32) -> Arc<FocusGeometry> {
    Arc::new(FocusGeometry(self.0.with_center_x(value)))
  }

  /// Vertical center of the focus area as a fraction of the image height. Defaults to 0.5.
  pub fn with_center_y(&self, value: f32) -> Arc<FocusGeometry> {
    Arc::new(FocusGeometry(self.0.with_center_y(value)))
  }

  /// Distance from the center to where the blur reaches full strength. Defaults to 0.75.
  pub fn with_radius(&self, value: f32) -> Arc<FocusGeometry> {
    Arc::new(FocusGeometry(self.0.with_radius(value)))
  }

  /// Fraction of the radius that stays fully sharp. 1 is a hard edge. Defaults to 0.25.
  pub fn with_sharpness(&self, value: f32) -> Arc<FocusGeometry> {
    Arc::new(FocusGeometry(self.0.with_sharpness(value)))
  }

  /// Where in the fade the blur reaches half strength. Lower values blur sooner. Defaults to 0.5.
  pub fn with_midpoint(&self, value: f32) -> Arc<FocusGeometry> {
    Arc::new(FocusGeometry(self.0.with_midpoint(value)))
  }

  /// Flattens the shape vertically: 0 is round, values toward 1 are flatter. Defaults to 0.
  pub fn with_aspect_ratio(&self, value: f32) -> Arc<FocusGeometry> {
    Arc::new(FocusGeometry(self.0.with_aspect_ratio(value)))
  }

  /// Clockwise rotation of the shape in degrees. Defaults to 0.
  pub fn with_rotation(&self, value: f32) -> Arc<FocusGeometry> {
    Arc::new(FocusGeometry(self.0.with_rotation(value)))
  }
}

/// The blur applied outside the focus area of a focus blur.
#[derive(Clone, uniffi::Enum)]
pub enum BlurType {
  Gaussian(Arc<GaussianBlur>),
  Lens(Arc<LensBlur>),
}

impl From<BlurType> for blur::BlurType {
  fn from(blur_type: BlurType) -> Self {
    match blur_type {
      BlurType::Gaussian(blur) => blur::BlurType::Gaussian(blur.0.clone()),
      BlurType::Lens(blur) => blur::BlurType::Lens(blur.0.clone()),
    }
  }
}

/// Blurs the image outside a focus area. Made by [`focus_blur`].
#[derive(uniffi::Object)]
pub struct FocusBlur(blur::FocusBlur);

/// A focus blur. By default the focus is a circle in the center and the blur is `gaussian_blur(25)`.
#[uniffi::export]
pub fn focus_blur() -> Arc<FocusBlur> {
  Arc::new(FocusBlur(blur::focus_blur()))
}

#[uniffi::export]
impl FocusBlur {
  /// Where the focus area sits and how its edge fades.
  pub fn with_geometry(&self, geometry: Arc<FocusGeometry>) -> Arc<FocusBlur> {
    Arc::new(FocusBlur(self.0.clone().with_geometry(geometry.0)))
  }

  /// The shape of the focus area. Defaults to a circle.
  pub fn with_shape(&self, shape: FocusShape) -> Arc<FocusBlur> {
    Arc::new(FocusBlur(self.0.clone().with_shape(shape)))
  }

  /// The blur applied outside the focus area, such as a gaussian or a lens blur.
  pub fn with_blur(&self, blur: BlurType) -> Arc<FocusBlur> {
    Arc::new(FocusBlur(self.0.clone().with_blur(blur::BlurType::from(blur))))
  }
}

/// Pinches or bulges the image. Made by [`pinch`].
#[derive(uniffi::Object)]
pub struct Pinch(distort::Pinch);

/// A pinch of this amount.
#[uniffi::export]
pub fn pinch(amount: f32) -> Arc<Pinch> {
  Arc::new(Pinch(distort::pinch(amount)))
}

/// Ripples the image. Made by [`ripple`].
#[derive(uniffi::Object)]
pub struct Ripple(distort::Ripple);

/// A ripple of this amount.
#[uniffi::export]
pub fn ripple(amount: f32) -> Arc<Ripple> {
  Arc::new(Ripple(distort::ripple(amount)))
}

#[uniffi::export]
impl Ripple {
  /// The size of the ripples.
  pub fn with_size(&self, size: RippleSize) -> Arc<Ripple> {
    Arc::new(Ripple(self.0.clone().with_size(size)))
  }

  /// The shape of the ripples.
  pub fn with_shape(&self, shape: RippleShape) -> Arc<Ripple> {
    Arc::new(Ripple(self.0.clone().with_shape(shape)))
  }
}

/// Finds the edges and makes them glow against a dark background. Made by [`glowing_edges`].
#[derive(uniffi::Object)]
pub struct GlowingEdges(edges::GlowingEdges);

/// A glowing edges effect.
#[uniffi::export]
pub fn glowing_edges() -> Arc<GlowingEdges> {
  Arc::new(GlowingEdges(edges::glowing_edges()))
}

#[uniffi::export]
impl GlowingEdges {
  /// The width of the detected edges. Defaults to 2.
  pub fn with_edge_width(&self, edge_width: u32) -> Arc<GlowingEdges> {
    Arc::new(GlowingEdges(self.0.clone().with_edge_width(edge_width)))
  }

  /// How bright the edges glow. Defaults to 6.
  pub fn with_edge_brightness(&self, edge_brightness: u32) -> Arc<GlowingEdges> {
    Arc::new(GlowingEdges(self.0.clone().with_edge_brightness(edge_brightness)))
  }

  /// How much the edges are smoothed. Defaults to 5.
  pub fn with_smoothness(&self, smoothness: u32) -> Arc<GlowingEdges> {
    Arc::new(GlowingEdges(self.0.clone().with_smoothness(smoothness)))
  }
}

/// Adds noise to the image. Made by [`noise`].
#[derive(uniffi::Object)]
pub struct Noise(noise::Noise);

/// Noise of this amount.
#[uniffi::export]
pub fn noise(amount: f32) -> Arc<Noise> {
  Arc::new(Noise(noise::noise(amount)))
}

#[uniffi::export]
impl Noise {
  /// How the noise is spread.
  pub fn with_distribution(&self, distribution: NoiseDistribution) -> Arc<Noise> {
    Arc::new(Noise(self.0.clone().with_distribution(distribution)))
  }
}

/// Removes specks. Made by [`despeckle`].
#[derive(uniffi::Object)]
pub struct Despeckle(noise::Despeckle);

/// A despeckle with this radius, which replaces pixels that differ from their neighbors by more than `threshold`.
#[uniffi::export]
pub fn despeckle(radius: f32, threshold: f32) -> Arc<Despeckle> {
  Arc::new(Despeckle(noise::despeckle(radius, threshold)))
}

/// Replaces each pixel with the median of its neighbors. Made by [`median`].
#[derive(uniffi::Object)]
pub struct Median(noise::Median);

/// A median with this radius.
#[uniffi::export]
pub fn median(radius: f32) -> Arc<Median> {
  Arc::new(Median(noise::median(radius)))
}

/// Sharpens the image. Made by [`sharpen`].
#[derive(uniffi::Object)]
pub struct Sharpen(sharpen_module::Sharpen);

/// A sharpen effect.
#[uniffi::export]
pub fn sharpen() -> Arc<Sharpen> {
  Arc::new(Sharpen(sharpen_module::sharpen()))
}

/// Smooths the image. Made by [`smooth`].
#[derive(uniffi::Object)]
pub struct Smooth(smooth_module::Smooth);

/// A smooth effect.
#[uniffi::export]
pub fn smooth() -> Arc<Smooth> {
  Arc::new(Smooth(smooth_module::smooth()))
}

/// Grows or shrinks the bright parts of an image. Made by [`dilate`] and [`erode`].
#[derive(uniffi::Object)]
pub struct Morphology(morphology::Morphology);

/// Grows each bright part of an image by this many pixels, each color channel on its own. The alpha is left as it was.
#[uniffi::export]
pub fn dilate(radius: u32) -> Arc<Morphology> {
  Arc::new(Morphology(morphology::dilate(radius)))
}

/// Shrinks each bright part of an image by this many pixels, each color channel on its own. The alpha is left as it was.
#[uniffi::export]
pub fn erode(radius: u32) -> Arc<Morphology> {
  Arc::new(Morphology(morphology::erode(radius)))
}

/// Makes a picture of a mask follow the edges of a photo, on the CPU or the GPU. Made by [`guided`].
#[derive(uniffi::Object)]
pub struct Guided(smooth_module::Guided);

/// A guided filter for a picture of a mask (white where it reaches) that makes it follow the edges of the brightness of `guide`.
/// `radius` is how far in pixels it looks and `epsilon` how strong an edge has to be (brightness counted from 0 to 1): `0.0001`
/// follows faint edges, `0.01` only strong ones. The result is an opaque gray picture. On the GPU it keeps its numbers in 32-bit
/// float textures, so use a picture no larger than it needs to be.
#[uniffi::export]
pub fn guided(guide: Arc<Image>, radius: u32, epsilon: f32) -> Arc<Guided> {
  let guide: CoreImage = guide.core();
  Arc::new(Guided(smooth_module::guided(&guide, radius, epsilon)))
}

/// A mask made to follow the edges of a photo: where the photo has a real edge the mask changes there, and where the photo is
/// flat the mask is smoothed. The result is the size of the photo; a mask of another size is first scaled to it. `radius` is how far
/// in pixels the filter looks, and `epsilon` how strong an edge has to be (colors counted from 0 to 1) before the mask follows it:
/// `0.0001` follows faint edges, `0.01` only strong ones.
#[uniffi::export]
pub fn guided_filter(mask: Arc<Mask>, guide: Arc<Image>, radius: u32, epsilon: f32) -> Arc<Mask> {
  let guide: CoreImage = guide.core();
  Arc::new(Mask::from_core(smooth_module::guided_filter(&mask.core(), &guide, radius, epsilon)))
}

/// Applies the Sobel kernel. Made by [`sobel`].
#[derive(uniffi::Object)]
pub struct Sobel(sobel_module::Sobel);

/// A Sobel filter in this direction.
#[uniffi::export]
pub fn sobel(direction: SobelDirection) -> Arc<Sobel> {
  Arc::new(Sobel(sobel_module::sobel(direction)))
}

/// The strength of the Sobel edges at each sampled pixel of the image, sampling every `step` pixels.
#[uniffi::export]
pub fn sobel_magnitude(image: Arc<Image>, step: u32) -> Vec<f32> {
  let image: CoreImage = image.core();
  sobel_module::sobel_magnitude(&image, step as usize)
}

effect_methods!(
  Blur,
  BoxBlur,
  GaussianBlur,
  LensBlur,
  MotionBlur,
  SurfaceBlur,
  FocusBlur,
  Pinch,
  Ripple,
  GlowingEdges,
  Noise,
  Despeckle,
  Median,
  Sharpen,
  Smooth,
  Sobel,
  Morphology,
  Guided,
);

#[cfg(test)]
mod tests {
  use super::*;
  use abra::abra_core::Channels;
  use abra::filters::prelude::Effect;

  const WIDTH: u32 = 16;
  const HEIGHT: u32 = 12;

  /// A varied picture with edges, so blurs, sharpening and edge filters all have something to do.
  fn picture() -> Vec<u8> {
    (0..WIDTH * HEIGHT)
      .flat_map(|i| {
        let (x, y) = (i % WIDTH, i / WIDTH);
        let block = ((x / 4 + y / 3) % 2) as u8 * 180;
        [
          block.wrapping_add((x * 7) as u8),
          (255 - y * 15) as u8,
          ((x * y) % 256) as u8,
          if i % 11 == 0 { 140 } else { 255 },
        ]
      })
      .collect()
  }

  fn new_image() -> Arc<Image> {
    Arc::new(Image::new_from_pixels(WIDTH, HEIGHT, picture(), Channels::RGBA))
  }

  /// What a handle's `apply_on_cpu` makes of the picture, and what the library's own effect makes of it.
  fn handle_and_library(handle: impl FnOnce(Arc<Image>), library: impl FnOnce(&mut CoreImage)) -> (Vec<u8>, Vec<u8>) {
    let image = new_image();
    handle(image.clone());
    let mut expected = CoreImage::new_from_pixels(WIDTH, HEIGHT, picture(), Channels::RGBA);
    library(&mut expected);
    (image.rgba(), expected.rgba().to_vec())
  }

  #[test]
  fn every_filter_gives_the_pixels_the_library_gives() {
    let cases: Vec<(&str, (Vec<u8>, Vec<u8>))> = vec![
      ("blur", handle_and_library(|i| blur().apply_on_cpu(i), |e| blur::blur().apply_on_cpu(e))),
      ("box_blur", handle_and_library(|i| box_blur(2.0).apply_on_cpu(i), |e| blur::box_blur(2.0).apply_on_cpu(e))),
      (
        "gaussian_blur",
        handle_and_library(|i| gaussian_blur(2.0).apply_on_cpu(i), |e| blur::gaussian_blur(2.0).apply_on_cpu(e)),
      ),
      (
        "lens_blur",
        handle_and_library(
          |i| {
            lens_blur(2.0)
              .with_shape(ApertureShape::Pentagon)
              .with_blade_curvature(0.3)
              .with_rotation(20.0)
              .with_specular(1.5, 0.6)
              .with_noise(0.1, NoiseDistribution::Gaussian)
              .with_samples(8)
              .apply_on_cpu(i)
          },
          |e| {
            blur::lens_blur(2.0)
              .with_shape(ApertureShape::Pentagon)
              .with_blade_curvature(0.3)
              .with_rotation(20.0)
              .with_specular(1.5, 0.6)
              .with_noise(0.1, NoiseDistribution::Gaussian)
              .with_samples(8)
              .apply_on_cpu(e)
          },
        ),
      ),
      (
        "motion_blur",
        handle_and_library(|i| motion_blur(30.0, 4).apply_on_cpu(i), |e| blur::motion_blur(30.0, 4).apply_on_cpu(e)),
      ),
      (
        "surface_blur",
        handle_and_library(
          |i| surface_blur(2, 30).with_step(2).apply_on_cpu(i),
          |e| blur::surface_blur(2, 30).with_step(2).apply_on_cpu(e),
        ),
      ),
      (
        "focus_blur",
        handle_and_library(
          |i| {
            focus_blur()
              .with_geometry(
                FocusGeometry::new()
                  .with_center_x(0.3)
                  .with_center_y(0.6)
                  .with_radius(0.5)
                  .with_sharpness(0.4)
                  .with_midpoint(0.4)
                  .with_aspect_ratio(0.2)
                  .with_rotation(15.0),
              )
              .with_shape(FocusShape::Diamond)
              .with_blur(BlurType::Gaussian(gaussian_blur(3.0)))
              .apply_on_cpu(i)
          },
          |e| {
            blur::focus_blur()
              .with_geometry(
                blur::FocusGeometry::new()
                  .with_center_x(0.3)
                  .with_center_y(0.6)
                  .with_radius(0.5)
                  .with_sharpness(0.4)
                  .with_midpoint(0.4)
                  .with_aspect_ratio(0.2)
                  .with_rotation(15.0),
              )
              .with_shape(FocusShape::Diamond)
              .with_blur(blur::gaussian_blur(3.0))
              .apply_on_cpu(e)
          },
        ),
      ),
      (
        "focus_blur with a lens blur",
        handle_and_library(
          |i| focus_blur().with_blur(BlurType::Lens(lens_blur(2.0).with_samples(4))).apply_on_cpu(i),
          |e| blur::focus_blur().with_blur(blur::lens_blur(2.0).with_samples(4)).apply_on_cpu(e),
        ),
      ),
      ("pinch", handle_and_library(|i| pinch(0.5).apply_on_cpu(i), |e| distort::pinch(0.5).apply_on_cpu(e))),
      (
        "ripple",
        handle_and_library(
          |i| ripple(0.4).with_size(RippleSize::Large).with_shape(RippleShape::Angle(30.0)).apply_on_cpu(i),
          |e| distort::ripple(0.4).with_size(RippleSize::Large).with_shape(RippleShape::Angle(30.0)).apply_on_cpu(e),
        ),
      ),
      (
        "glowing_edges",
        handle_and_library(
          |i| glowing_edges().with_edge_width(3).with_edge_brightness(8).with_smoothness(2).apply_on_cpu(i),
          |e| edges::glowing_edges().with_edge_width(3).with_edge_brightness(8).with_smoothness(2).apply_on_cpu(e),
        ),
      ),
      (
        "noise",
        handle_and_library(
          |i| noise(0.3).with_distribution(NoiseDistribution::Gaussian).apply_on_cpu(i),
          |e| noise::noise(0.3).with_distribution(NoiseDistribution::Gaussian).apply_on_cpu(e),
        ),
      ),
      (
        "despeckle",
        handle_and_library(|i| despeckle(1.0, 10.0).apply_on_cpu(i), |e| noise::despeckle(1.0, 10.0).apply_on_cpu(e)),
      ),
      ("median", handle_and_library(|i| median(1.0).apply_on_cpu(i), |e| noise::median(1.0).apply_on_cpu(e))),
      ("dilate", handle_and_library(|i| dilate(2).apply_on_cpu(i), |e| morphology::dilate(2).apply_on_cpu(e))),
      ("erode", handle_and_library(|i| erode(2).apply_on_cpu(i), |e| morphology::erode(2).apply_on_cpu(e))),
      (
        "guided",
        handle_and_library(
          |i| guided(new_image(), 2, 0.001).apply_on_cpu(i),
          |e| {
            let guide = CoreImage::new_from_pixels(WIDTH, HEIGHT, picture(), Channels::RGBA);
            smooth_module::guided(&guide, 2, 0.001).apply_on_cpu(e)
          },
        ),
      ),
      ("sharpen", handle_and_library(|i| sharpen().apply_on_cpu(i), |e| sharpen_module::sharpen().apply_on_cpu(e))),
      ("smooth", handle_and_library(|i| smooth().apply_on_cpu(i), |e| smooth_module::smooth().apply_on_cpu(e))),
      (
        "sobel horizontal",
        handle_and_library(
          |i| sobel(SobelDirection::Horizontal).apply_on_cpu(i),
          |e| sobel_module::sobel(SobelDirection::Horizontal).apply_on_cpu(e),
        ),
      ),
      (
        "sobel vertical",
        handle_and_library(
          |i| sobel(SobelDirection::Vertical).apply_on_cpu(i),
          |e| sobel_module::sobel(SobelDirection::Vertical).apply_on_cpu(e),
        ),
      ),
    ];
    for (name, (made, expected)) in cases {
      assert_eq!(made, expected, "{name} differs from the library");
    }
  }

  #[test]
  fn the_filters_change_the_picture() {
    // A guard for the test above: a handle that did nothing would also equal a library effect that did nothing.
    let changed = |apply: &dyn Fn(Arc<Image>)| {
      let image = new_image();
      apply(image.clone());
      image.rgba() != picture()
    };
    assert!(changed(&|i| gaussian_blur(3.0).apply_on_cpu(i)), "gaussian_blur left the picture as it was");
    assert!(changed(&|i| noise(0.5).apply_on_cpu(i)), "noise left the picture as it was");
    assert!(changed(&|i| sobel(SobelDirection::Horizontal).apply_on_cpu(i)), "sobel left the picture as it was");
    assert!(changed(&|i| sharpen().apply_on_cpu(i)), "sharpen left the picture as it was");
  }

  #[test]
  fn apply_picks_the_hardware_and_always_works() {
    let image = new_image();
    gaussian_blur(2.0).apply(image.clone());
    assert_ne!(image.rgba(), picture());
  }

  #[test]
  #[should_panic(expected = "this operation has no GPU version")]
  fn a_filter_without_a_shader_cannot_be_applied_on_the_gpu() {
    sharpen().apply_on_gpu(new_image());
  }

  #[test]
  fn the_guided_filter_is_the_librarys() {
    let library = CoreImage::new_from_pixels(WIDTH, HEIGHT, picture(), Channels::RGBA);
    let values: Vec<u8> = (0..WIDTH * HEIGHT).map(|i| if i % WIDTH < WIDTH / 2 { 0 } else { 255 }).collect();
    let made = guided_filter(Arc::new(Mask::from_values(WIDTH, HEIGHT, values.clone())), new_image(), 3, 0.001);
    let expected = smooth_module::guided_filter(&abra::mask::prelude::Mask::from_values(WIDTH, HEIGHT, values), &library, 3, 0.001);
    assert_eq!(made.values(), expected.values());
  }

  #[test]
  fn the_sobel_magnitude_is_the_librarys() {
    let library = CoreImage::new_from_pixels(WIDTH, HEIGHT, picture(), Channels::RGBA);
    assert_eq!(sobel_magnitude(new_image(), 2), sobel_module::sobel_magnitude(&library, 2));
  }
}
