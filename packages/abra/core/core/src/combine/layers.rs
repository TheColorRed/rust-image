//! The steps that stacking layers needs besides laying one picture over another (see [`recipe::lay_over`](crate::image::recipe::lay_over)): limiting a
//! picture to a mask, mixing what an adjustment layer's effects made into what was beneath it, and holding a layer's live effects as steps of a
//! picture that waits for the GPU.
//!
//! Each is an effect with a CPU version and a GPU pass that give the same pixels, so a stack of layers is one chain that runs on the GPU when there is
//! one, with nothing copied back to the CPU in between, and is made on the CPU when there is not.

use std::sync::{Arc, OnceLock};

use crate::image::GrayPlane;
use crate::image::effect::{Effect, Options};
use crate::image::gpu::{CpuProcessor, GpuAux, GpuPass, GpuProcessor, LiveEffect};
use crate::image::recipe::{effect_applied, gpu_ready};
use crate::Image;

/// A mask as the GPU is given it: the values, and a texture of them (each value in the red, green and blue channels) made once, so a mask that does
/// not change is uploaded once and not each time the picture it limits is drawn.
#[derive(Clone, Debug)]
pub struct MaskTexture {
  plane: GrayPlane,
  rgba: Arc<[u8]>,
}

impl MaskTexture {
  pub fn new(p_plane: GrayPlane) -> MaskTexture {
    let rgba: Vec<u8> = p_plane.values().iter().flat_map(|value| [*value, *value, *value, 255]).collect();
    MaskTexture { plane: p_plane, rgba: Arc::from(rgba) }
  }

  /// The values of the mask.
  pub fn plane(&self) -> &GrayPlane {
    &self.plane
  }

  fn aux(&self) -> GpuAux {
    GpuAux { width: self.plane.width(), height: self.plane.height(), rgba: Arc::clone(&self.rgba) }
  }
}

/// Multiplies the alpha of a picture by a mask the size of the picture: white keeps what is there, black hides it, and what is already transparent
/// stays so.
#[derive(Clone)]
struct MaskAlpha {
  mask: Arc<MaskTexture>,
  options: Options,
}

impl Effect for MaskAlpha {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  /// The mask is read where the pixel is in the picture, so the effect cannot be run on a part of it by itself.
  fn positional(&self) -> bool {
    true
  }

  fn cpu_processor(&self, p_image: &mut Image) {
    let (width, height) = p_image.dimensions::<u32>();
    if self.mask.plane.dimensions::<u32>() != (width, height) {
      return;
    }
    let mut pixels = p_image.to_rgba_vec();
    for (rgba, value) in pixels.chunks_exact_mut(4).zip(self.mask.plane.values()) {
      rgba[3] = ((rgba[3] as u16 * *value as u16 + 127) / 255) as u8;
    }
    p_image.set_rgba(pixels);
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
  }
}

impl GpuProcessor for MaskAlpha {
  fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<GpuPass> {
    vec![GpuPass::new(include_str!("./mask_alpha.wgsl"), []).with_window().with_aux(self.mask.aux())]
  }
}

/// What an adjustment layer does: its effects are applied to everything beneath it, and the result is mixed into what was there by a strength (the
/// layer's opacity) and a mask, whose top left corner is at `origin` in the picture.
#[derive(Clone)]
struct Adjust {
  effects: Vec<Arc<dyn LiveEffect>>,
  strength: f32,
  mask: Option<Arc<MaskTexture>>,
  origin: (i32, i32),
  options: Options,
}

impl Effect for Adjust {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn positional(&self) -> bool {
    true
  }

  fn cpu_processor(&self, p_image: &mut Image) {
    let mut adjusted = p_image.clone();
    for effect in &self.effects {
      effect.apply_cpu(&mut adjusted);
    }
    let (width, height) = p_image.dimensions::<u32>();
    let adjusted_pixels = adjusted.to_rgba_vec();
    let mut pixels = p_image.to_rgba_vec();
    for y in 0..height {
      for x in 0..width {
        let mask_value = match &self.mask {
          Some(mask) => {
            let (mask_x, mask_y) = (x as i32 - self.origin.0, y as i32 - self.origin.1);
            if mask_x < 0 || mask_y < 0 { 0 } else { mask.plane.get(mask_x as u32, mask_y as u32).unwrap_or(0) }
          }
          None => 255,
        };
        let strength = mask_value as f32 / 255.0 * self.strength;
        let at = (y * width + x) as usize * 4;
        for channel in 0..4 {
          let (from, to) = (pixels[at + channel] as f32, adjusted_pixels[at + channel] as f32);
          pixels[at + channel] = (from + (to - from) * strength).round() as u8;
        }
      }
    }
    p_image.set_rgba(pixels);
  }

  /// The GPU draws it when every one of the effects can.
  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    self.effects.iter().all(|effect| effect.has_gpu()).then_some(self)
  }
}

impl GpuProcessor for Adjust {
  fn passes(&self, p_width: u32, p_height: u32) -> Vec<GpuPass> {
    let mut passes: Vec<GpuPass> = self.effects.iter().flat_map(|effect| effect.passes(p_width, p_height)).collect();
    // Without a mask the texture is a white pixel that is not read, so the pass has the same bindings either way.
    static NO_MASK: OnceLock<Arc<[u8]>> = OnceLock::new();
    let aux = match &self.mask {
      Some(mask) => mask.aux(),
      None => GpuAux { width: 1, height: 1, rgba: Arc::clone(NO_MASK.get_or_init(|| Arc::from([255u8; 4].as_slice()))) },
    };
    let uniforms = [
      self.strength.to_le_bytes(),
      u32::from(self.mask.is_some()).to_le_bytes(),
      self.origin.0.to_le_bytes(),
      self.origin.1.to_le_bytes(),
    ]
    .concat();
    // The last pass mixes the effects' result with the picture as it was before the first of them (`with_base`).
    passes.push(GpuPass::new(include_str!("./adjust_mix.wgsl"), uniforms).with_window().with_base().with_aux(aux));
    passes
  }
}

/// A live effect held as a step of a picture that waits for the GPU.
struct LiveStep(Arc<dyn LiveEffect>);

impl CpuProcessor for LiveStep {
  fn process(&self, p_image: &mut Image) {
    self.0.apply_cpu(p_image);
  }

  fn gpu(&self) -> Option<&dyn GpuProcessor> {
    self.0.has_gpu().then_some(self)
  }
}

impl GpuProcessor for LiveStep {
  fn passes(&self, p_width: u32, p_height: u32) -> Vec<GpuPass> {
    self.0.passes(p_width, p_height)
  }
}

/// Limits `p_image` to a mask the size of the picture, in place: white keeps what is there, black hides it. With a GPU the picture is deferred, and
/// the mask is one more step of the chain; without one it is done on the pixels now.
pub fn multiply_alpha(p_image: &mut Image, p_mask: &Arc<MaskTexture>) {
  let step = MaskAlpha { mask: Arc::clone(p_mask), options: None };
  if gpu_ready() {
    *p_image = effect_applied(p_image, Arc::new(step));
  } else {
    step.cpu_processor(p_image);
  }
}

/// What an adjustment layer does to `p_image`, in place: applies `p_effects` to all of it and mixes the result in by `p_strength` (0 to 1) and `p_mask`,
/// whose top left corner is at `p_origin` in the picture (where the mask does not reach nothing is changed). With a GPU the picture is deferred and the
/// effects and the mix are steps of the chain; without one, or when an effect has no shader, it is done on the pixels when they are needed.
pub fn adjust(p_image: &mut Image, p_effects: &[Arc<dyn LiveEffect>], p_strength: f32, p_mask: Option<&Arc<MaskTexture>>, p_origin: (i32, i32)) {
  let strength = p_strength.clamp(0.0, 1.0);
  if p_effects.is_empty() || strength == 0.0 {
    return;
  }
  let step = Adjust { effects: p_effects.to_vec(), strength, mask: p_mask.cloned(), origin: p_origin, options: None };
  if gpu_ready() {
    *p_image = effect_applied(p_image, Arc::new(step));
  } else {
    step.cpu_processor(p_image);
  }
}

/// `p_image` with a live effect applied, as a picture that waits for the GPU when there is one (and is made now when there is not), so a layer's
/// effects are steps of the chain and not a separate run whose pixels are copied back.
pub fn live_effect_applied(p_image: &Image, p_effect: Arc<dyn LiveEffect>) -> Image {
  if gpu_ready() {
    effect_applied(p_image, Arc::new(LiveStep(p_effect)))
  } else {
    let mut result = p_image.clone();
    p_effect.apply_cpu(&mut result);
    result
  }
}

/// The color of a flat shape that keeps the outline of a picture (a shadow, say): every pixel gets the color, an alpha is kept as it is unless it is above
/// `solid_above`, and then it is fully opaque.
#[derive(Clone)]
struct Silhouette {
  color: [u8; 3],
  solid_above: u8,
  options: Options,
}

impl Effect for Silhouette {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn cpu_processor(&self, p_image: &mut Image) {
    let mut pixels = p_image.to_rgba_vec();
    for rgba in pixels.chunks_exact_mut(4) {
      let alpha = if rgba[3] > self.solid_above { 255 } else { rgba[3] };
      rgba.copy_from_slice(&[self.color[0], self.color[1], self.color[2], alpha]);
    }
    p_image.set_rgba(pixels);
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
  }
}

impl GpuProcessor for Silhouette {
  fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<GpuPass> {
    let uniforms = [self.color[0] as u32, self.color[1] as u32, self.color[2] as u32, self.solid_above as u32].map(u32::to_le_bytes).concat();
    vec![GpuPass::new(include_str!("./silhouette.wgsl"), uniforms)]
  }
}

/// The alpha of a picture multiplied by a factor, the fraction dropped.
#[derive(Clone)]
struct AlphaScale {
  factor: f32,
  options: Options,
}

impl Effect for AlphaScale {
  fn options(&self) -> &Options {
    &self.options
  }

  fn options_mut(&mut self) -> &mut Options {
    &mut self.options
  }

  fn cpu_processor(&self, p_image: &mut Image) {
    let mut pixels = p_image.to_rgba_vec();
    for rgba in pixels.chunks_exact_mut(4) {
      rgba[3] = (rgba[3] as f32 * self.factor) as u8;
    }
    p_image.set_rgba(pixels);
  }

  fn gpu_processor(&self) -> Option<&dyn GpuProcessor> {
    Some(self)
  }
}

impl GpuProcessor for AlphaScale {
  fn passes(&self, _p_width: u32, _p_height: u32) -> Vec<GpuPass> {
    vec![GpuPass::new(include_str!("./alpha_scale.wgsl"), self.factor.to_le_bytes())]
  }
}

/// `p_image` turned into a flat `p_color` shape with its outline (a shadow's shape): an alpha above `p_solid_above` becomes fully opaque and a lower one is
/// kept. A picture that waits for the GPU when there is one, and is made now when there is not.
pub fn silhouette(p_image: &Image, p_color: [u8; 3], p_solid_above: u8) -> Image {
  let step = Silhouette { color: p_color, solid_above: p_solid_above, options: None };
  if gpu_ready() {
    effect_applied(p_image, Arc::new(step))
  } else {
    let mut result = p_image.clone();
    step.cpu_processor(&mut result);
    result
  }
}

/// Multiplies the alpha of `p_image` by `p_factor`, in place, dropping the fraction. With a GPU it is one more step of the picture's chain.
pub fn scale_alpha(p_image: &mut Image, p_factor: f32) {
  let step = AlphaScale { factor: p_factor, options: None };
  if gpu_ready() {
    *p_image = effect_applied(p_image, Arc::new(step));
  } else {
    step.cpu_processor(p_image);
  }
}
