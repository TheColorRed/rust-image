use crate::common::*;

#[napi]
/// Applies a Gaussian blur to the image.
/// @param layer The layer to apply the Gaussian blur to.
/// @param radius The radius of the Gaussian blur.
/// @param options Optional apply options for masking and area.
pub fn gaussian_blur(layer: &mut Layer, radius: f64, options: Option<&ApplyOptions>) {
  let options = options.unwrap_or(&ApplyOptions::default()).to_apply_options();
  layer.get_underlying_layer().with_image_mut(|img| {
    blur::gaussian_blur(radius as u32).with_options(options).apply(img);
  });
  layer.mark_dirty();
}
#[napi]
/// Applies a surface blur to the image.
/// @param layer The layer to apply the surface blur to.
/// @param radius The radius of the surface blur.
/// @param threshold The threshold for the surface blur.
/// @param options Optional apply options for masking and area.
pub fn surface_blur(layer: &mut Layer, radius: f64, threshold: f64, options: Option<&ApplyOptions>) {
  let options = options.unwrap_or(&ApplyOptions::default()).to_apply_options();
  layer.get_underlying_layer().with_image_mut(|img| {
    blur::surface_blur(radius as u32, threshold as u8).with_options(options).apply(img);
  });
  layer.mark_dirty();
}
#[napi]
/// Blurs the image using a box blur algorithm.
/// @param layer The layer to apply the box blur to.
/// @param radius The radius of the box blur.
/// @param options Optional apply options for masking and area.
pub fn box_blur(layer: &mut Layer, radius: f64, options: Option<&ApplyOptions>) {
  let options = options.unwrap_or(&ApplyOptions::default()).to_apply_options();
  layer.get_underlying_layer().with_image_mut(|img| {
    blur::box_blur(radius).with_options(options).apply(img);
  });
  layer.mark_dirty();
}

#[napi(object)]
/// Options for iris (aperture) in lens blur.
pub struct IrisOptions {
  #[napi(ts_type = "'triangle' | 'square' | 'pentagon' | 'hexagon' | 'heptagon' | 'octagon'")]
  /// Shape of the aperture
  pub shape: String,
  /// Radius of the aperture
  pub radius: u32,
  /// Curvature of the blades (0.0 = straight, 1.0 = fully curved)
  pub blade_curvature: f64,
  /// Rotation of the aperture in degrees
  pub rotation: f64,
}
#[napi(object)]
/// Options for specular highlights in lens blur.
pub struct SpecularOptions {
  /// Brightness multiplier for specular highlights.
  pub brightness: f64,
  /// Threshold for specular highlights (0.0 - 1.0).
  pub threshold: f64,
}
#[napi(object)]
/// Options for noise added after lens blur.
pub struct NoiseOptions {
  /// Amount of noise to add.
  pub amount: f64,
  #[napi(ts_type = "'uniform' | 'gaussian'")]
  /// Distribution type: "uniform" or "gaussian".
  pub distribution: String,
}

#[napi(object)]
/// Options for lens blur filter.
pub struct LensBlurOptions {
  /// Iris (aperture) configuration.
  pub iris: IrisOptions,
  /// Specular highlight boost; None to disable.
  pub specular: Option<SpecularOptions>,
  /// Output noise/dither; None to disable.
  pub noise: Option<NoiseOptions>,
  /// Number of samples per pixel. Higher is smoother but slower.
  pub samples: u32,
}

#[napi]
/// Applies a lens blur to the image with specified options.
/// @param layer The layer to apply the lens blur to.
/// @param options Configuration options for the lens blur.
/// @param apply_options Optional apply options for masking and area.
pub fn lens_blur(layer: &mut Layer, options: Option<LensBlurOptions>, apply_options: Option<&ApplyOptions>) {
  use abra::filters::prelude::blur::*;
  let mut lens_blur = blur::lens_blur(options.as_ref().map_or(0, |opts| opts.iris.radius));
  if let Some(opts) = options {
    let shape = match opts.iris.shape.as_str() {
      "triangle" => ApertureShape::Triangle,
      "square" => ApertureShape::Square,
      "pentagon" => ApertureShape::Pentagon,
      "hexagon" => ApertureShape::Hexagon,
      "heptagon" => ApertureShape::Heptagon,
      "octagon" => ApertureShape::Octagon,
      _ => ApertureShape::Hexagon,
    };
    lens_blur = lens_blur
      .with_shape(shape)
      .with_blade_curvature(opts.iris.blade_curvature as f32)
      .with_rotation(opts.iris.rotation as f32)
      .with_samples(opts.samples);
    if let Some(specular) = opts.specular {
      lens_blur = lens_blur.with_specular(specular.brightness as f32, specular.threshold as f32);
    }
    if let Some(noise) = opts.noise {
      let distribution = match noise.distribution.as_str() {
        "gaussian" => NoiseDistribution::Gaussian,
        _ => NoiseDistribution::Uniform,
      };
      lens_blur = lens_blur.with_noise(noise.amount as f32, distribution);
    }
  }

  let apply_options = apply_options.unwrap_or(&ApplyOptions::default()).to_apply_options();
  layer.get_underlying_layer().with_image_mut(|img| {
    lens_blur.with_options(apply_options).apply(img);
  });
  layer.mark_dirty();
}

#[napi]
/// Applies a motion blur to the image.
/// @param layer The layer to apply the motion blur to.
/// @param angle The angle of the motion blur in degrees.
/// @param distance The distance of the motion blur in pixels.
pub fn motion_blur(layer: &mut Layer, angle: f64, distance: f64, options: Option<&ApplyOptions>) {
  let options = options.unwrap_or(&ApplyOptions::default()).to_apply_options();
  layer.get_underlying_layer().with_image_mut(|img| {
    blur::motion_blur(angle as f32, distance as u32).with_options(options).apply(img);
  });
  layer.mark_dirty();
}
