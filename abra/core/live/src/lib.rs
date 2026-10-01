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
mod masked;

use abra_core::image::gpu::{GpuAux, LiveEffect};
use abra_core::{Channels, Image};
use masked::MaskedEffect;
use options::{Effect, ApplyTarget, Options};
use std::sync::Arc;

/// A rendered preview frame.
#[derive(Clone, Debug)]
pub struct LiveFrame {
  /// Width in pixels.
  pub width: u32,
  /// Height in pixels.
  pub height: u32,
  /// `width * height` RGBA pixels.
  pub pixels: Vec<u8>,
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
  #[cfg(feature = "gpu")]
  presenter: Option<gpu::Presenter>,
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
      #[cfg(feature = "gpu")]
      presenter: None,
    })
  }

  #[cfg(feature = "gpu")]
  fn backend_for(p_width: u32, p_height: u32, p_rgba: &[u8]) -> Backend {
    if abra_core::Settings::gpu_enabled()
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

  /// Draws frames straight onto an Android `ANativeWindow` instead of returning them from [`poll`](Self::poll), so no
  /// pixels are copied to the CPU. Only the GPU backend can present, so this fails on the CPU backend.
  ///
  /// # Safety
  /// `p_window` must be a valid `ANativeWindow` that stays valid until [`stop_presenting`](Self::stop_presenting) is
  /// called or the preview is dropped.
  pub unsafe fn present_to_android_window(
    &mut self, p_window: *mut std::ffi::c_void, p_width: u32, p_height: u32,
  ) -> Result<(), String> {
    #[cfg(feature = "gpu")]
    {
      if !self.is_gpu() {
        return Err("presenting to a surface needs the GPU".to_string());
      }
      let context = gpu::context().ok_or("no GPU adapter available")?;
      let presenter = unsafe { gpu::Presenter::from_android_window(context, p_window, p_width, p_height) }
        .map_err(|e| e.to_string())?;
      self.presenter = Some(presenter);
      self.render();
      Ok(())
    }
    #[cfg(not(feature = "gpu"))]
    {
      let _ = (p_window, p_width, p_height);
      Err("presenting to a surface needs the gpu feature".to_string())
    }
  }

  /// Goes back to returning frames from [`poll`](Self::poll).
  pub fn stop_presenting(&mut self) {
    #[cfg(feature = "gpu")]
    if self.presenter.take().is_some() {
      self.render();
    }
  }

  /// Tells the presenter its window changed size.
  pub fn resize_surface(&mut self, p_width: u32, p_height: u32) {
    #[cfg(feature = "gpu")]
    if let Some(presenter) = &mut self.presenter {
      presenter.resize(p_width, p_height);
    }
    #[cfg(not(feature = "gpu"))]
    let _ = (p_width, p_height);
  }

  /// Whether frames are drawn onto a surface.
  pub fn is_presenting(&self) -> bool {
    #[cfg(feature = "gpu")]
    return self.presenter.is_some();
    #[cfg(not(feature = "gpu"))]
    false
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
    let ctx = options::get_ctx(p_options.as_ref()).filter(|ctx| ctx.area.is_some() || ctx.mask_image.is_some())?;
    let weights = abra_core::image::apply_area::area_weights(self.width, self.height, &ctx);
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

  /// Renders the chain. A failure is reported by the next [`poll`](Self::poll).
  fn render(&mut self) {
    let effects: Vec<Arc<dyn LiveEffect>> = self.chain.iter().map(|entry| entry.effect.clone()).collect();
    let result = match &mut self.backend {
      #[cfg(feature = "gpu")]
      Backend::Gpu(session) => match &mut self.presenter {
        Some(presenter) => session.present(&effects, presenter),
        None => session.submit(effects),
      },
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

  /// The newest finished frame, if there is one since the last call. Never blocks.
  pub fn poll(&mut self) -> Result<Option<LiveFrame>, String> {
    if let Some(message) = self.error.take() {
      return Err(message);
    }
    match &mut self.backend {
      #[cfg(feature = "gpu")]
      Backend::Gpu(_) if self.presenter.is_some() => Ok(None),
      #[cfg(feature = "gpu")]
      Backend::Gpu(session) => Ok(session.poll()?.map(|frame| LiveFrame {
        width: frame.width,
        height: frame.height,
        pixels: frame.pixels,
      })),
      Backend::Cpu { ready } => Ok(ready.take()),
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
  use abra_core::Area;
  use options::ApplyOptions;

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
