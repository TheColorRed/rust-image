use crate::common::*;

#[napi]
/// Applies a Gaussian blur to the image.
/// @param layer The layer to apply the Gaussian blur to.
/// @param radius The radius of the Gaussian blur.
/// @param options Optional apply options for masking and area.
pub fn gaussian_blur(layer: &mut Layer, radius: f64, options: Option<&ApplyOptions>) {
  let layer_ref = layer.get_underlying_layer_mut();
  let options = options.unwrap_or(&ApplyOptions::default()).to_apply_options();
  blur::gaussian_blur(&mut *layer_ref, radius as u32, options);
  layer.mark_dirty();
}
#[napi]
/// Applies a surface blur to the image.
/// @param layer The layer to apply the surface blur to.
/// @param radius The radius of the surface blur.
/// @param threshold The threshold for the surface blur.
/// @param options Optional apply options for masking and area.
pub fn surface_blur(layer: &mut Layer, radius: f64, threshold: f64, options: Option<&ApplyOptions>) {
  let layer_ref = layer.get_underlying_layer_mut();
  let options = options.unwrap_or(&ApplyOptions::default()).to_apply_options();
  blur::surface_blur(&mut *layer_ref, radius as u32, threshold as u8, options);
  layer.mark_dirty();
}
#[napi]
/// Blurs the image using a box blur algorithm.
/// @param layer The layer to apply the box blur to.
/// @param radius The radius of the box blur.
/// @param options Optional apply options for masking and area.
pub fn box_blur(layer: &mut Layer, radius: f64, options: Option<&ApplyOptions>) {
  let layer_ref = layer.get_underlying_layer_mut();
  let options = options.unwrap_or(&ApplyOptions::default()).to_apply_options();
  blur::box_blur(&mut *layer_ref, radius as f64, options);
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
  let layer_ref = layer.get_underlying_layer_mut();
  let iris = match &options {
    Some(opts) => {
      let shape = match opts.iris.shape.as_str() {
        "triangle" => ApertureShape::Triangle,
        "square" => ApertureShape::Square,
        "pentagon" => ApertureShape::Pentagon,
        "hexagon" => ApertureShape::Hexagon,
        "heptagon" => ApertureShape::Heptagon,
        "octagon" => ApertureShape::Octagon,
        _ => ApertureShape::Hexagon,
      };
      IrisOptions {
        shape,
        radius: opts.iris.radius,
        blade_curvature: opts.iris.blade_curvature as f32,
        rotation: opts.iris.rotation.to_radians() as f32,
      }
    }
    None => IrisOptions::default(),
  };
  let specular = options.as_ref().and_then(|opts| {
    opts.specular.as_ref().map(|spec_opts| SpecularOptions {
      brightness: spec_opts.brightness as f32,
      threshold: spec_opts.threshold as f32,
    })
  });
  let noise = options.as_ref().and_then(|opts| {
    opts.noise.as_ref().map(|noise_opts| {
      let distribution = match noise_opts.distribution.as_str() {
        "uniform" => NoiseDistribution::Uniform,
        "gaussian" => NoiseDistribution::Gaussian,
        _ => NoiseDistribution::Uniform,
      };
      NoiseOptions {
        amount: noise_opts.amount as f32,
        distribution,
      }
    })
  });
  let blur_options = LensBlurOptions {
    iris,
    specular,
    noise,
    samples: options.as_ref().map_or(0, |opts| opts.samples),
  };

  let apply_options = apply_options.unwrap_or(&ApplyOptions::default()).to_apply_options();
  blur::lens_blur(&mut *layer_ref, blur_options, apply_options);
  layer.mark_dirty();
}

#[napi]
/// Applies a motion blur to the image.
/// @param layer The layer to apply the motion blur to.
/// @param angle The angle of the motion blur in degrees.
/// @param distance The distance of the motion blur in pixels.
pub fn motion_blur(layer: &mut Layer, angle: f64, distance: f64, options: Option<&ApplyOptions>) {
  let layer_ref = layer.get_underlying_layer_mut();
  let options = options.unwrap_or(&ApplyOptions::default()).to_apply_options();
  blur::motion_blur(&mut *layer_ref, angle as f32, distance as u32, options);
  layer.mark_dirty();
}
