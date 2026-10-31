use abra::filters::prelude::blur::gaussian_blur;
use abra::filters::prelude::smooth::guided_filter_plane;
use abra::adjustments::prelude::Effect;
use abra::abra_core::color::{LumaStandard, luma};
use abra::prelude::*;
use rayon::prelude::*;

mod light;
mod lights;

pub mod prelude {
  pub use super::Lighting;
  pub use super::light::{Incident, Light, PositionedLight};
  pub use super::lights::{AreaLight, DirectionalLight, OmniLight, SpotLight};
}

use light::Light;

/// The most a pixel's brightness can differ from its surroundings and still count as texture, from `0` to `1`.
const TEXTURE_LIMIT: f32 = 0.12;

/// Shines lights on a picture. The picture has no depth of its own, so the shape of the scene comes from a depth map
/// (`1` is near, `0` is far, as the depth model makes it): the way each part faces is worked out from how the depth
/// changes around it, and each light then brightens the parts that face it.
pub struct Lighting {
  lights: Vec<Box<dyn Light>>,
  ambient: f32,
  depth_scale: f32,
  smoothing: f32,
  depth_edges: f32,
  max_slope: f32,
  detail: f32,
  detail_size: f32,
  shadows: f32,
  shadow_softness: f32,
  shadow_reach: f32,
  shadow_steps: u32,
}

impl Lighting {
  /// Lighting with no lights, which leaves the picture as it is (an ambient of 1).
  pub fn new() -> Self {
    Self {
      lights: Vec::new(),
      ambient: 1.0,
      depth_scale: 0.15,
      smoothing: 0.003,
      depth_edges: 0.02,
      max_slope: 0.6,
      detail: 3.0,
      detail_size: 0.002,
      shadows: 0.0,
      shadow_softness: 0.1,
      shadow_reach: 0.3,
      shadow_steps: 32,
    }
  }

  /// Adds a light.
  pub fn with_light(mut self, p_light: impl Light + 'static) -> Self {
    self.lights.push(Box::new(p_light));
    self
  }

  /// Sets how much of the picture's own brightness stays where no light reaches, from `0` (black) to `1` (unchanged).
  pub fn with_ambient(mut self, p_ambient: f32) -> Self {
    self.ambient = p_ambient.max(0.0);
    self
  }

  /// Sets how deep the scene is, from its nearest to its farthest part, as a share of the picture's longest side. A deeper
  /// scene has steeper slopes, so the lights show more shape.
  pub fn with_depth_scale(mut self, p_depth_scale: f32) -> Self {
    self.depth_scale = p_depth_scale;
    self
  }

  /// Sets how much the depth is smoothed before its slopes are measured, as a share of the picture's longest side.
  pub fn with_smoothing(mut self, p_smoothing: f32) -> Self {
    self.smoothing = p_smoothing.max(0.0);
    self
  }

  /// Sets how strong an edge in the photo has to be, with colors counted from `0` to `1`, before the depth is kept sharp along it.
  /// Small values (`0.0001`) follow even faint edges; larger ones (`0.01`) follow only strong edges and smooth the rest.
  pub fn with_depth_edges(mut self, p_epsilon: f32) -> Self {
    self.depth_edges = p_epsilon.max(f32::EPSILON);
    self
  }

  /// Sets the steepest slope a surface can have, as a rise over a run (`1` is 45 degrees). A steeper slope is held to this, so no
  /// part of a picture ends up facing completely away from every light. A limit of `0` leaves the surface flat.
  pub fn with_max_slope(mut self, p_max_slope: f32) -> Self {
    self.max_slope = p_max_slope.max(0.0);
    self
  }

  /// Sets how much the picture's own texture (brick, fabric, skin, leaves) catches the light, as the height in pixels of its
  /// brightest ridge over its darkest groove. The depth only knows the large shapes of a scene, so a flat wall would light
  /// evenly; the texture gives it a surface. `0` leaves it out.
  pub fn with_detail(mut self, p_detail: f32) -> Self {
    self.detail = p_detail.max(0.0);
    self
  }

  /// Sets how coarse the texture is that catches the light, as a share of the picture's longest side: bumps smaller than this
  /// count as texture and anything larger is left to the depth.
  pub fn with_detail_size(mut self, p_detail_size: f32) -> Self {
    self.detail_size = p_detail_size.max(0.0);
    self
  }

  /// Makes the scene block light: a part that is in front of another throws a shadow on it, away from the light. The depth is
  /// treated as a landscape, and a ray is followed from each point toward each light; if the landscape rises above the ray, the
  /// light is blocked. `0` (the default) turns shadows off, which is quicker; `1` makes them fully dark.
  /// - `p_opacity`: How dark the shadows are, from `0` to `1`.
  pub fn with_shadows(mut self, p_opacity: f32) -> Self {
    self.shadows = p_opacity.clamp(0.0, 1.0);
    self
  }

  /// Sets how soft the edge of a shadow is, as how much wider it gets for each pixel it is cast: `0` is hard-edged and a larger
  /// number is a bigger light, with a wider fade at the edge.
  pub fn with_shadow_softness(mut self, p_softness: f32) -> Self {
    self.shadow_softness = p_softness.max(f32::EPSILON);
    self
  }

  /// Sets how far a shadow can be cast, as a share of the picture's longest side. Shadows that would be longer are cut off.
  pub fn with_shadow_reach(mut self, p_reach: f32) -> Self {
    self.shadow_reach = p_reach.max(0.0);
    self
  }

  /// Sets how many points are checked along each ray. More catches thinner things but takes longer.
  pub fn with_shadow_steps(mut self, p_steps: u32) -> Self {
    self.shadow_steps = p_steps.max(1);
    self
  }

  /// Shines the lights on a picture.
  /// - `p_image`: The picture, which is changed.
  /// - `p_depth`: How near each pixel is, from `0` (the farthest part of the scene) to `1` (the nearest), one number for each
  ///   pixel of the picture, row by row from the top left. A depth model's `DepthMap` scaled to the picture gives this.
  pub fn apply(&self, p_image: &mut Image, p_depth: &[f32]) {
    let (width, height) = p_image.dimensions::<usize>();
    assert_eq!(p_depth.len(), width * height, "the depth needs one number for each pixel of the picture");
    let longest = width.max(height) as f32;

    // Smooth the depth along the photo's own edges: it stays sharp where the photo has an edge (the outline of a person) and is
    // smoothed where the photo is flat, so the depth neither bleeds past an outline nor shows noise.
    let radius = (longest * self.smoothing).round().max(1.0) as u32;
    let depth = guided_filter_plane(p_depth, p_image, radius, self.depth_edges);
    // Slopes are measured across a few pixels, so a bit of noise left in the depth does not read as a bump.
    let reach = radius as usize;
    let scale = self.depth_scale * longest;
    let heights: Vec<f32> = depth.iter().map(|value| value * scale).collect();
    let at = |x: usize, y: usize| heights[y.min(height - 1) * width + x.min(width - 1)];

    // The texture is how much brighter each pixel is than the picture blurred to the size of the texture's bumps.
    let detail_radius = (longest * self.detail_size).round().max(1.0) as u32;
    let fine: Vec<f32> = if self.detail > 0.0 {
      let mut blurred = p_image.clone();
      gaussian_blur(detail_radius).apply(&mut blurred);
      let brightness = |rgba: &[u8]| luma(rgba[0] as f32, rgba[1] as f32, rgba[2] as f32, LumaStandard::Rec601);
      p_image
        .rgba()
        .chunks_exact(4)
        .zip(blurred.rgba().chunks_exact(4))
        // A big jump, such as the outline of a person against a wall, is an edge and not a texture: left to count in full
        // it would show as a thin ridge drawn around everything, so only small differences count.
        .map(|(sharp, soft)| ((brightness(sharp) - brightness(soft)) / 255.0).clamp(-TEXTURE_LIMIT, TEXTURE_LIMIT) * self.detail)
        .collect()
    } else {
      Vec::new()
    };
    let bump = |x: usize, y: usize| if fine.is_empty() { 0.0 } else { fine[y.min(height - 1) * width + x.min(width - 1)] };

    // The slope of the height at every pixel, measured across a few pixels so a bit of noise in the depth does not read as a bump.
    let mut slopes: Vec<[f32; 2]> = vec![[0.0; 2]; width * height];
    slopes.par_chunks_mut(width).enumerate().for_each(|(y, row)| {
      for (x, slope) in row.iter_mut().enumerate() {
        *slope = [
          (at(x + reach, y) - at(x.saturating_sub(reach), y)) / (2 * reach) as f32,
          (at(x, y + reach) - at(x, y.saturating_sub(reach))) / (2 * reach) as f32,
        ];
      }
    });
    // Along an outline (a person against a wall) the depth jumps, and the slope there is the edge between two things and not a
    // surface that faces anywhere. It is steep for a band of pixels either side, so each pixel takes the flattest slope within
    // a few pixels of it: next to an edge there is always flat wall or flat skin close by, and the whole band goes flat with no
    // ring left behind. A slope that is real (a rounded body) is the same all round, so it is kept.
    let magnitude = |p_slope: [f32; 2]| p_slope[0] * p_slope[0] + p_slope[1] * p_slope[1];
    let window = reach * 3;
    let flattest = |p_slopes: &[[f32; 2]], p_from: usize, p_step: usize, p_count: usize, p_index: usize| {
      let mut best = p_slopes[p_from + p_index * p_step];
      for other in p_index.saturating_sub(window)..(p_index + window + 1).min(p_count) {
        let candidate = p_slopes[p_from + other * p_step];
        if magnitude(candidate) < magnitude(best) {
          best = candidate;
        }
      }
      best
    };
    // Taking the flattest slope in a square window leaves square patches where it changes, so the result is averaged a little to
    // round them off.
    let mean = |p_slopes: &[[f32; 2]], p_from: usize, p_step: usize, p_count: usize, p_index: usize| {
      let (from, to) = (p_index.saturating_sub(reach), (p_index + reach + 1).min(p_count));
      let sum = (from..to).fold([0.0f32; 2], |sum, other| {
        let slope = p_slopes[p_from + other * p_step];
        [sum[0] + slope[0], sum[1] + slope[1]]
      });
      [sum[0] / (to - from) as f32, sum[1] / (to - from) as f32]
    };
    let across: Vec<[f32; 2]> = (0..width * height)
      .into_par_iter()
      .map(|i| flattest(&slopes, (i / width) * width, 1, width, i % width))
      .collect();
    let down: Vec<[f32; 2]> =
      (0..width * height).into_par_iter().map(|i| flattest(&across, i % width, width, height, i / width)).collect();
    let across: Vec<[f32; 2]> =
      (0..width * height).into_par_iter().map(|i| mean(&down, (i / width) * width, 1, width, i % width)).collect();
    let slopes: Vec<[f32; 2]> =
      (0..width * height).into_par_iter().map(|i| mean(&across, i % width, width, height, i / width)).collect();

    let shadow_reach = longest * self.shadow_reach;
    let (width_f, height_f) = (width as f32, height as f32);

    let source = p_image.rgba();
    let mut pixels = vec![0u8; source.len()];
    pixels.par_chunks_exact_mut(width * 4).enumerate().for_each(|(y, row)| {
      for (x, out) in row.chunks_exact_mut(4).enumerate() {
        // The slope of the height across the pixel, and so the way the surface faces, with the picture's own texture on top.
        let slope = slopes[y * width + x];
        let (dx, dy) = (
          slope[0] + (bump(x + 1, y) - bump(x.saturating_sub(1), y)) * 0.5,
          slope[1] + (bump(x, y + 1) - bump(x, y.saturating_sub(1))) * 0.5,
        );
        let steepness = (dx * dx + dy * dy).sqrt();
        let limit = if steepness > self.max_slope { self.max_slope / steepness } else { 1.0 };
        let (dx, dy) = (dx * limit, dy * limit);
        let length = (dx * dx + dy * dy + 1.0).sqrt();
        let normal = [-dx / length, -dy / length, 1.0 / length];

        let mut shine = [self.ambient; 3];
        for light in &self.lights {
          let incident = light.illuminate([x as f32, y as f32, at(x, y)]);
          let facing = (normal[0] * incident.direction[0] + normal[1] * incident.direction[1] + normal[2] * incident.direction[2]).max(0.0);
          // How much of the light gets through: follow a ray toward the light and see how close the scene comes to it.
          let mut visibility = 1.0f32;
          if self.shadows > 0.0 && facing > 0.0 {
            let step = incident.distance.min(shadow_reach) / self.shadow_steps as f32;
            let start = at(x, y);
            for i in 1..=self.shadow_steps {
              let t = step * i as f32;
              let (ray_x, ray_y) = (x as f32 + incident.direction[0] * t, y as f32 + incident.direction[1] * t);
              if ray_x < 0.0 || ray_y < 0.0 || ray_x >= width_f || ray_y >= height_f {
                break;
              }
              let gap = start + incident.direction[2] * t - at(ray_x as usize, ray_y as usize);
              visibility = visibility.min((gap / (self.shadow_softness * t)).clamp(0.0, 1.0));
              if visibility <= 0.0 {
                break;
              }
            }
          }
          let lit = 1.0 - self.shadows * (1.0 - visibility);
          for channel in 0..3 {
            shine[channel] += incident.color[channel] * incident.strength * facing * lit;
          }
        }

        let from = (y * width + x) * 4;
        for channel in 0..3 {
          out[channel] = (source[from + channel] as f32 * shine[channel]).round().clamp(0.0, 255.0) as u8;
        }
        out[3] = source[from + 3];
      }
    });
    *p_image = Image::new_from_pixels(width as u32, height as u32, pixels, Channels::RGBA);
  }
}

impl Default for Lighting {
  fn default() -> Self {
    Self::new()
  }
}
