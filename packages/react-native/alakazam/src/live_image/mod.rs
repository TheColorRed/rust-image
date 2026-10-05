//! Interactive rendering: hold an image, change effect parameters, get frames back without blocking.
//!
//! Effects are applied to a [`LiveImage`] with the same call as to an image: `brightness(30).apply(&mut image)`
//! edits the image now, `brightness(30).apply(&mut preview)` re-renders the preview. Whether the GPU or CPU does the
//! work is decided here. With the `gpu` feature and a usable GPU, effects run as shaders over an image uploaded once;
//! otherwise (or when the GPU is disabled in settings) the same effects run on the CPU.
//!
//! Programs that only edit images one at a time (a command line tool, a batch job) do not need this crate.

#[cfg(feature = "gpu")]
mod gpu_backend;
#[cfg(test)]
mod live_image_tests;
mod masked;

// The old live-image path keeps its own name for the engine's frame type.
pub use vessel::Frame as LiveFrame;

use abra::abra_core::image::gpu::{GpuAux, LiveEffect};
use abra::abra_core::{Channels, Image};
use abra_body_segmentation::Mask;
use abra::options::prelude::{ApplyTarget, Effect, Options};
use masked::MaskedEffect;
use std::sync::Arc;

// `EffectSink` is abra's trait for "something an effect can be applied to"; a live image is one.
impl abra::live::EffectSink for &mut LiveImage {
  fn accept<E: abra::live::SpecEffect>(self, p_effect: E) {
    p_effect.apply(self);
  }
}

impl abra::live::EffectSink for LiveSlot<'_> {
  fn accept<E: abra::live::SpecEffect>(self, p_effect: E) {
    p_effect.apply(self);
  }
}

enum Backend {
  #[cfg(feature = "gpu")]
  Gpu(gpu_backend::GpuBackend),
  Cpu {
    ready: Option<LiveFrame>,
  },
}

/// Names one effect in a [`LiveImage`]'s chain, so it can be changed, moved or removed later.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EffectId(pub u32);

struct Entry {
  id: EffectId,
  effect: Arc<dyn LiveEffect>,
  /// The area and mask the effect was given, kept with it so rendering can limit the effect to them.
  options: Options,
  /// The per-pixel weights built from `options`, if it has an area or mask. Kept so changing only the effect's
  /// parameters, such as while a slider is dragged, does not build them again.
  weights: Option<GpuAux>,
}

/// Whether two sets of options are known to be the same, so weights built from one fit the other.
fn same_options(p_a: &Options, p_b: &Options) -> bool {
  match (p_a, p_b) {
    (None, None) => true,
    (Some(a), Some(b)) => a.is_copy_of(b),
    _ => false,
  }
}

/// An image being edited interactively.
///
/// It holds the original image and an ordered chain of effects, and renders the chain over the original whenever it
/// changes. Applying an effect the way you would to an image, `brightness(30).apply(&mut live)`, adds it to the end
/// of the chain and gives it an [`EffectId`] (see [`new_id`](Self::new_id)). To change an effect that is already in the
/// chain, such as while a slider is dragged, apply it to its slot instead: `brightness(40).apply(live.slot(id))`
/// replaces the effect with that id and keeps its place. [`remove`](Self::remove) and [`move_to`](Self::move_to) take
/// an effect out or reorder it.
///
/// Call [`poll`](Self::poll) on a timer or display tick. On the GPU `apply` returns immediately and a slow consumer
/// only ever sees the newest state; on the CPU `apply` renders before returning, so call it from a worker thread.
/// An effect is limited to the area and mask it was given, with the same result as applying it to an image.
pub struct LiveImage {
  width: u32,
  height: u32,
  original: Vec<u8>,
  backend: Backend,
  chain: Vec<Entry>,
  next_id: u32,
  error: Option<String>,
  /// Set while [`batch`](Self::batch) runs, so changes inside it do not each render.
  batching: bool,
  /// Whether a chain made only of GPU effects is rendered to a texture for a surface to draw, instead of being read back.
  #[cfg(feature = "gpu")]
  direct: bool,
  /// The picture of the last render, when it was made as a texture. Taken with [`take_texture`](Self::take_texture).
  #[cfg(feature = "gpu")]
  texture: Option<gpu_passes::target::TextureFrame>,
}

impl LiveImage {
  /// Starts a preview of `p_rgba` (`p_width * p_height` RGBA pixels).
  pub fn new(p_width: u32, p_height: u32, p_rgba: Vec<u8>) -> Result<LiveImage, String> {
    if p_width == 0 || p_height == 0 || p_rgba.len() != p_width as usize * p_height as usize * 4 {
      return Err(format!("expected {p_width}x{p_height} RGBA pixels, got {} bytes", p_rgba.len()));
    }
    Ok(LiveImage {
      width: p_width,
      height: p_height,
      backend: Self::backend_for(p_width, p_height, &p_rgba),
      original: p_rgba,
      chain: Vec::new(),
      next_id: 0,
      error: None,
      batching: false,
      #[cfg(feature = "gpu")]
      direct: false,
      #[cfg(feature = "gpu")]
      texture: None,
    })
  }

  /// Runs `p_change`, then renders once. Every change inside it (adding, replacing, removing, clearing) would
  /// otherwise render the chain on its own, which for a chain with a CPU effect means computing it again for each one.
  /// Use it to replace a whole chain at once. A batch inside a batch renders when the outermost one ends.
  pub fn batch(&mut self, p_change: impl FnOnce(&mut LiveImage)) {
    let outer = self.batching;
    self.batching = true;
    p_change(self);
    self.batching = outer;
    if !outer {
      self.render();
    }
  }

  #[cfg(feature = "gpu")]
  fn backend_for(p_width: u32, p_height: u32, p_rgba: &[u8]) -> Backend {
    if abra::abra_core::Settings::gpu_enabled()
      && let Ok(backend) = gpu_backend::GpuBackend::new(p_width, p_height, p_rgba)
    {
      return Backend::Gpu(backend);
    }
    Backend::Cpu { ready: None }
  }

  #[cfg(not(feature = "gpu"))]
  fn backend_for(_p_width: u32, _p_height: u32, _p_rgba: &[u8]) -> Backend {
    Backend::Cpu { ready: None }
  }

  /// Whether frames are rendered on the GPU.
  pub fn is_gpu(&self) -> bool {
    !matches!(self.backend, Backend::Cpu { .. })
  }

  /// The size of the preview in pixels.
  pub fn size(&self) -> (u32, u32) {
    (self.width, self.height)
  }

  /// Chooses whether a chain made only of GPU effects is rendered to a texture (`true`), which a surface that can draw one
  /// takes with [`take_texture`](Self::take_texture) and draws without any copy to the CPU, or is read back into pixels
  /// for [`poll`](Self::poll) (`false`, the default). Renders the chain again in the new way.
  pub fn set_direct(&mut self, p_direct: bool) {
    #[cfg(feature = "gpu")]
    {
      self.direct = p_direct;
      self.render();
    }
    #[cfg(not(feature = "gpu"))]
    let _ = p_direct;
  }

  /// Whether chains of GPU effects are rendered as textures (see [`set_direct`](Self::set_direct)).
  pub fn is_direct(&self) -> bool {
    #[cfg(feature = "gpu")]
    return self.direct;
    #[cfg(not(feature = "gpu"))]
    false
  }

  /// The picture of the last render as a texture, if it was made as one (see [`set_direct`](Self::set_direct)).
  #[cfg(feature = "gpu")]
  pub fn take_texture(&mut self) -> Option<gpu_passes::target::TextureFrame> {
    self.texture.take()
  }

  /// Removes every effect, so the next frame is the original image.
  pub fn clear(&mut self) {
    self.chain.clear();
    self.render();
  }

  /// A fresh id that no effect in this image has, for adding an effect through [`slot`](Self::slot).
  pub fn new_id(&mut self) -> EffectId {
    let id = EffectId(self.next_id);
    self.next_id += 1;
    id
  }

  /// Where an effect with the given id lives. Applying an effect to it replaces the effect with that id, or adds it
  /// to the end of the chain if there is none.
  pub fn slot(&mut self, p_id: EffectId) -> LiveSlot<'_> {
    LiveSlot { image: self, id: p_id }
  }

  /// Takes the effect with the given id out of the chain. Returns whether there was one.
  pub fn remove(&mut self, p_id: EffectId) -> bool {
    let Some(position) = self.position(p_id) else { return false };
    self.chain.remove(position);
    self.render();
    true
  }

  /// Moves the effect with the given id to `p_index` in the chain (0 runs first; past the end means last). Returns
  /// whether there was one.
  pub fn move_to(&mut self, p_id: EffectId, p_index: usize) -> bool {
    let Some(position) = self.position(p_id) else { return false };
    let entry = self.chain.remove(position);
    self.chain.insert(p_index.min(self.chain.len()), entry);
    self.render();
    true
  }

  /// The ids of the effects in the order they run.
  pub fn ids(&self) -> Vec<EffectId> {
    self.chain.iter().map(|entry| entry.id).collect()
  }

  fn position(&self, p_id: EffectId) -> Option<usize> {
    self.chain.iter().position(|entry| entry.id == p_id)
  }

  /// The area and mask the effect with this id was given, or `None` if there is no such effect.
  pub fn options(&self, p_id: EffectId) -> Option<&Options> {
    self.position(p_id).map(|position| &self.chain[position].options)
  }

  /// The weights that limit an effect to the area and mask in `p_options`, or `None` if it has neither.
  fn weights_for(&self, p_options: &Options) -> Option<GpuAux> {
    let ctx = abra::options::prelude::get_ctx(p_options.as_ref()).filter(|ctx| ctx.area.is_some() || ctx.mask_image.is_some())?;
    let weights = abra::abra_core::image::apply_area::area_weights(self.width, self.height, &ctx);
    Some(masked::weights_texture(self.width, self.height, &weights))
  }

  fn put(&mut self, p_id: EffectId, p_effect: Arc<dyn LiveEffect>, p_options: Options) {
    self.next_id = self.next_id.max(p_id.0.saturating_add(1));
    let position = self.position(p_id);
    // Replacing an effect with one that has the same area and mask reuses the weights built for the old one.
    let weights = match position.map(|position| &self.chain[position]) {
      Some(entry) if same_options(&entry.options, &p_options) => entry.weights.clone(),
      _ => self.weights_for(&p_options),
    };
    let effect: Arc<dyn LiveEffect> = match &weights {
      Some(weights) => Arc::new(MaskedEffect::with_weights(p_effect, weights.clone())),
      None => p_effect,
    };
    let entry = Entry {
      id: p_id,
      effect,
      options: p_options,
      weights,
    };
    match position {
      Some(position) => self.chain[position] = entry,
      None => self.chain.push(entry),
    }
    self.render();
  }

  /// Renders the chain. A failure is reported by the next [`pixels`](Self::pixels).
  fn render(&mut self) {
    if self.batching {
      return;
    }
    let effects: Vec<Arc<dyn LiveEffect>> = self.chain.iter().map(|entry| entry.effect.clone()).collect();
    let result = match &mut self.backend {
      #[cfg(feature = "gpu")]
      Backend::Gpu(session) => {
        self.texture = None;
        // Rendered now only for a surface that draws textures; otherwise `pixels` renders when it is asked for.
        if self.direct {
          session.render_texture(&effects).map(|texture| self.texture = texture)
        } else {
          Ok(())
        }
      }
      Backend::Cpu { ready } => {
        let mut image = Image::new_from_pixels(self.width, self.height, self.original.clone(), Channels::RGBA);
        for effect in &effects {
          effect.apply_cpu(&mut image);
        }
        *ready = Some(LiveFrame {
          width: self.width,
          height: self.height,
          pixels: image.into_rgba_vec(),
        });
        Ok(())
      }
    };
    if let Err(message) = result {
      self.error = Some(message);
    }
  }

  /// The photo with `p_effects` applied, in order, replacing whatever chain was there. A texture when the surface draws
  /// textures when the GPU can make them, so the engine draws them without copying anything to the CPU (or reads one back
  /// itself if its surface can't show it), and pixels otherwise.
  pub fn picture<'a>(
    &mut self, p_effects: impl IntoIterator<Item = &'a crate::effect_spec::EffectSpec>,
  ) -> Option<vessel::Picture> {
    self.picture_with_skin_mask(p_effects, None)
  }

  pub(crate) fn picture_with_skin_mask<'a>(
    &mut self, p_effects: impl IntoIterator<Item = &'a crate::effect_spec::EffectSpec>, p_skin_mask: Option<&Mask>,
  ) -> Option<vessel::Picture> {
    if !self.is_direct() {
      self.set_direct(true);
    }
    self.batch(|live| {
      live.clear();
      for effect in p_effects {
        let id = live.new_id();
        effect.apply_with_skin_mask(live.slot(id), p_skin_mask);
      }
    });
    #[cfg(feature = "gpu")]
    if let Some(texture) = self.take_texture() {
      return Some(vessel::Picture::Gpu(Arc::new(texture)));
    }
    match self.pixels() {
      Ok(frame) => Some(vessel::Picture::Pixels(Arc::new(frame))),
      Err(message) => {
        eprintln!("[session] a frame could not be rendered: {message}");
        None
      }
    }
  }

  /// The picture as pixels: the chain rendered over the original, waiting for the GPU if the chain runs there. When a
  /// surface can draw a texture (see [`set_direct`](Self::set_direct)) take that instead, which copies nothing to the CPU.
  pub fn pixels(&mut self) -> Result<LiveFrame, String> {
    if let Some(message) = self.error.take() {
      return Err(message);
    }
    if matches!(self.backend, Backend::Cpu { ready: None }) {
      self.render();
    }
    let effects: Vec<Arc<dyn LiveEffect>> = self.chain.iter().map(|entry| entry.effect.clone()).collect();
    match &mut self.backend {
      #[cfg(feature = "gpu")]
      Backend::Gpu(session) => {
        let refs: Vec<&dyn LiveEffect> = effects.iter().map(|effect| effect.as_ref()).collect();
        Ok(LiveFrame {
          width: self.width,
          height: self.height,
          pixels: session.render_blocking(&refs)?,
        })
      }
      Backend::Cpu { ready } => ready.take().ok_or_else(|| "nothing has been rendered".to_string()),
    }
  }
}

/// A copy of the effect with its area and mask taken off, and those options. The chain limits the effect to them
/// itself, so the effect must not also apply them.
fn without_options<E: Effect + LiveEffect + Clone + 'static>(p_effect: &E) -> (Arc<dyn LiveEffect>, Options) {
  let options = p_effect.options().clone();
  let mut effect = p_effect.clone();
  *effect.options_mut() = None;
  (Arc::new(effect), options)
}

/// One place in a [`LiveImage`]'s chain, from [`LiveImage::slot`]. Applying an effect to it replaces the effect with
/// that id.
pub struct LiveSlot<'a> {
  image: &'a mut LiveImage,
  id: EffectId,
}

impl<E: Effect + LiveEffect + Clone + 'static> ApplyTarget<E> for LiveSlot<'_> {
  fn receive(self, p_effect: &E) {
    let (effect, options) = without_options(p_effect);
    self.image.put(self.id, effect, options);
  }
}

impl<E: Effect + LiveEffect + Clone + 'static> ApplyTarget<E> for &mut LiveImage {
  fn receive(self, p_effect: &E) {
    let id = self.new_id();
    let (effect, options) = without_options(p_effect);
    self.put(id, effect, options);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use abra::abra_core::Area;
  use abra::options::prelude::ApplyOptions;

  /// A do-nothing effect that carries options, to look at what the chain does with them.
  #[derive(Clone)]
  struct Probe {
    options: Options,
  }

  impl Effect for Probe {
    fn options(&self) -> &Options {
      &self.options
    }

    fn options_mut(&mut self) -> &mut Options {
      &mut self.options
    }

    fn cpu_processor(&self, _p_image: &mut Image) {}
  }

  fn live() -> LiveImage {
    LiveImage::new(8, 8, vec![10; 8 * 8 * 4]).unwrap()
  }

  fn weights(p_live: &LiveImage, p_id: EffectId) -> Option<GpuAux> {
    p_live.chain[p_live.position(p_id).unwrap()].weights.clone()
  }

  #[test]
  fn changing_only_the_effect_reuses_the_weights_built_for_its_area() {
    let mut live = live();
    let id = live.new_id();
    let options = ApplyOptions::new().with_area(Area::rect((1.0, 1.0), (4.0, 4.0)));
    let probe = Probe { options: Some(options) };

    probe.apply(live.slot(id));
    let first = weights(&live, id).expect("an area means weights");
    // Applying a copy of the same effect again, as a slider does on every tick, keeps the same texture.
    probe.clone().apply(live.slot(id));
    let again = weights(&live, id).unwrap();
    assert!(Arc::ptr_eq(&first.rgba, &again.rgba), "the weights were built again for the same options");
  }

  #[test]
  fn new_options_build_new_weights_and_no_options_drop_them() {
    let mut live = live();
    let id = live.new_id();
    let area = || ApplyOptions::new().with_area(Area::rect((1.0, 1.0), (4.0, 4.0)));

    Probe { options: Some(area()) }.apply(live.slot(id));
    let first = weights(&live, id).unwrap();
    // Options built separately may differ, even from equal values, so the weights are built again.
    Probe { options: Some(area()) }.apply(live.slot(id));
    let second = weights(&live, id).unwrap();
    assert!(!Arc::ptr_eq(&first.rgba, &second.rgba));
    assert_eq!(first.rgba, second.rgba, "and they come out the same");

    Probe { options: None }.apply(live.slot(id));
    assert!(weights(&live, id).is_none());
  }

  #[test]
  fn an_effect_without_an_area_or_mask_has_no_weights() {
    let mut live = live();
    let id = live.new_id();
    Probe { options: Some(ApplyOptions::new()) }.apply(live.slot(id));
    assert!(weights(&live, id).is_none());
  }
}
