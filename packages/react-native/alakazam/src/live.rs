use std::sync::{Arc, Mutex};

use abra::live::{EffectId, EffectSpec, LiveImage};

use crate::{
  AbraError, AbraImage,
  image::PreviewImage,
  surface::{self, Window},
};

/// The preview and the window it draws on. The preview is declared first so it drops before the window it draws on.
struct State {
  preview: LiveImage,
  attached: Option<Attached>,
  wanted: Option<i32>,
}

struct Attached {
  id: i32,
  window: Arc<Window>,
  size: (u32, u32),
}

impl State {
  /// Attaches to the wanted window once its view has reported a surface.
  fn try_attach(&mut self) {
    let Some(id) = self.wanted else { return };
    let Some(window) = surface::get(id) else { return };
    let (width, height, ok) = {
      let state = window.state.lock().unwrap();
      let ok = state.alive
        && unsafe { self.preview.present_to_android_window(window.ptr(), state.width, state.height) }.is_ok();
      (state.width, state.height, ok)
    };
    if ok {
      self.attached = Some(Attached {
        id,
        window,
        size: (width, height),
      });
    }
  }

  /// Runs `p_change` on the preview with its window locked, so the platform can't destroy the surface mid-frame.
  /// A window that is gone drops the preview back to polled frames.
  fn run(&mut self, p_change: impl FnOnce(&mut LiveImage)) {
    if self.attached.is_none() {
      self.try_attach();
    }
    let Some(attached) = &mut self.attached else { return p_change(&mut self.preview) };
    let window = attached.window.clone();
    let state = window.state.lock().unwrap();
    let current = surface::get(attached.id).is_some_and(|w| Arc::ptr_eq(&w, &window));
    if !state.alive || !current {
      drop(state);
      self.preview.stop_presenting();
      self.attached = None;
      return p_change(&mut self.preview);
    }
    if attached.size != (state.width, state.height) {
      attached.size = (state.width, state.height);
      self.preview.resize_surface(state.width, state.height);
    }
    p_change(&mut self.preview);
  }
}

/// An image being edited interactively. Add effects with `addEffect` and call `setEffect` on every parameter change. Without a surface, collect frames
/// with `poll` on a timer or frame callback; with one (see `attachSurface`), frames are drawn on it directly.
#[derive(uniffi::Object)]
pub struct AbraLiveImage {
  inner: Mutex<State>,
}

#[uniffi::export]
impl AbraLiveImage {
  /// Starts a preview of `image` downscaled to fit `max_width` x `max_height`, so a drag renders at screen size.
  #[uniffi::constructor]
  pub fn new(image: &AbraImage, max_width: u32, max_height: u32) -> Result<Arc<Self>, AbraError> {
    let proxy = image.preview(max_width, max_height);
    let preview =
      LiveImage::new(proxy.width, proxy.height, proxy.data).map_err(|message| AbraError::Render { message })?;
    Ok(Arc::new(Self {
      inner: Mutex::new(State {
        preview,
        attached: None,
        wanted: None,
      }),
    }))
  }

  /// Whether frames are rendered on the GPU (otherwise on the CPU, which blocks this call).
  pub fn is_gpu(&self) -> bool {
    self.inner.lock().unwrap().preview.is_gpu()
  }

  /// The preview's width in pixels.
  pub fn width(&self) -> u32 {
    self.inner.lock().unwrap().preview.size().0
  }

  /// The preview's height in pixels.
  pub fn height(&self) -> u32 {
    self.inner.lock().unwrap().preview.size().1
  }

  /// Draws frames on the native preview view registered under `surface_id` instead of returning them from `poll`.
  /// The view may appear after this call; drawing starts with the next effect change. Returns whether the preview
  /// is drawing on the surface already.
  pub fn attach_surface(&self, surface_id: i32) -> bool {
    let mut state = self.inner.lock().unwrap();
    if state.wanted != Some(surface_id) {
      state.preview.stop_presenting();
      state.attached = None;
      state.wanted = Some(surface_id);
    }
    state.run(|_| {});
    state.attached.is_some()
  }

  /// Goes back to returning frames from `poll`.
  pub fn detach_surface(&self) {
    let mut state = self.inner.lock().unwrap();
    state.wanted = None;
    state.preview.stop_presenting();
    state.attached = None;
  }

  /// Whether frames are being drawn on a surface.
  pub fn is_presenting(&self) -> bool {
    self.inner.lock().unwrap().preview.is_presenting()
  }

  /// Adds an effect to the end of the chain and returns its id. Use the id with `setEffect`, `removeEffect` and
  /// `moveEffect`.
  pub fn add_effect(&self, effect: EffectSpec) -> u32 {
    let mut state = self.inner.lock().unwrap();
    let mut id = EffectId(0);
    state.run(|live| {
      id = live.new_id();
      effect.apply(live.slot(id));
    });
    id.0
  }

  /// Replaces the effect with this id, keeping its place in the chain. Call it on every slider change. An id that is
  /// not in the chain adds the effect at the end.
  pub fn set_effect(&self, id: u32, effect: EffectSpec) {
    self.inner.lock().unwrap().run(|live| effect.apply(live.slot(EffectId(id))));
  }

  /// Removes the effect with this id. Returns whether there was one.
  pub fn remove_effect(&self, id: u32) -> bool {
    let mut removed = false;
    self.inner.lock().unwrap().run(|live| removed = live.remove(EffectId(id)));
    removed
  }

  /// Moves the effect with this id to `index` in the chain (0 runs first). Returns whether there was one.
  pub fn move_effect(&self, id: u32, index: u32) -> bool {
    let mut moved = false;
    self.inner.lock().unwrap().run(|live| moved = live.move_to(EffectId(id), index as usize));
    moved
  }

  /// Removes every effect.
  pub fn clear(&self) {
    self.inner.lock().unwrap().run(|preview| preview.clear());
  }

  /// The newest finished frame, or nothing. Never blocks. Always nothing while drawing on a surface.
  pub fn poll(&self) -> Result<Option<PreviewImage>, AbraError> {
    let frame = self.inner.lock().unwrap().preview.poll().map_err(|message| AbraError::Render { message })?;
    Ok(frame.map(|frame| PreviewImage {
      width: frame.width,
      height: frame.height,
      data: frame.pixels,
    }))
  }
}

#[uniffi::export]
impl AbraImage {
  /// Applies one effect to the image. Uses the same definition as live previews.
  pub fn apply_effect(&self, effect: EffectSpec) {
    self.with_image_mut(|img| effect.apply(img));
  }
}
