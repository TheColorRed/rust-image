//! The Layer public API struct.

use abra_core::Image;
use abra_core::image::image_ext::GuardedOwner;
use std::sync::Arc;
use std::sync::Mutex;

use crate::canvas::layer_inner::LayerInner;
use crate::effects::LayerEffects;
use abra_core::blend::RGBA;
use abra_core::image::image_ext::ImageRef;
use std::sync::MutexGuard;

pub use super::anchor::Anchor;
pub use super::layer_transform::LayerTransform;
pub use super::origin::Origin;

#[derive(Debug, Clone, Copy)]
pub enum AdjustmentLayerType {
  BrightnessContrast,
  Exposure,
  HueSaturation,
  ColorBalance,
  BlackWhite,
  PhotoFilter,
  ChannelMixer,
  Invert,
  Posterize,
  GradientMap,
  SolidColor,
  Gradient,
  Pattern,
}

impl AdjustmentLayerType {
  pub fn to_string(&self) -> String {
    let s = match self {
      AdjustmentLayerType::BrightnessContrast => "brightness-contrast",
      AdjustmentLayerType::Exposure => "exposure",
      AdjustmentLayerType::HueSaturation => "hue-saturation",
      AdjustmentLayerType::ColorBalance => "color-balance",
      AdjustmentLayerType::BlackWhite => "black-white",
      AdjustmentLayerType::PhotoFilter => "photo-filter",
      AdjustmentLayerType::ChannelMixer => "channel-mixer",
      AdjustmentLayerType::Invert => "invert",
      AdjustmentLayerType::Posterize => "posterize",
      AdjustmentLayerType::GradientMap => "gradient-map",
      AdjustmentLayerType::SolidColor => "solid-color",
      AdjustmentLayerType::Gradient => "gradient",
      AdjustmentLayerType::Pattern => "pattern",
    };
    s.to_string()
  }
}

impl From<String> for AdjustmentLayerType {
  fn from(s: String) -> Self {
    match s.as_str() {
      "brightness-contrast" => AdjustmentLayerType::BrightnessContrast,
      "exposure" => AdjustmentLayerType::Exposure,
      "hue-saturation" => AdjustmentLayerType::HueSaturation,
      "color-balance" => AdjustmentLayerType::ColorBalance,
      "black-white" => AdjustmentLayerType::BlackWhite,
      "photo-filter" => AdjustmentLayerType::PhotoFilter,
      "channel-mixer" => AdjustmentLayerType::ChannelMixer,
      "invert" => AdjustmentLayerType::Invert,
      "posterize" => AdjustmentLayerType::Posterize,
      "gradient-map" => AdjustmentLayerType::GradientMap,
      "solid-color" => AdjustmentLayerType::SolidColor,
      "gradient" => AdjustmentLayerType::Gradient,
      "pattern" => AdjustmentLayerType::Pattern,
      _ => AdjustmentLayerType::BrightnessContrast, // Default case
    }
  }
}

/// A layer in a project.
/// This is the public API struct that wraps `Arc<Mutex<LayerInner>>`.
#[derive(Debug)]
pub struct Layer {
  /// Reference to the inner layer.
  inner_layer: Arc<Mutex<LayerInner>>,
}

impl Layer {
  /// Creates a new layer with the given name and image.
  pub fn new(name: impl Into<String>, image: Arc<Image>) -> Self {
    Layer {
      inner_layer: Arc::new(Mutex::new(LayerInner::new(name, image))),
    }
  }

  /// Returns an ImageRef that provides access to the image and keeps the
  /// inner mutex guard alive for as long as the ImageRef is alive.
  /// This avoids cloning the underlying image data and ensures the lock is
  /// held while callers access the image.
  pub fn image(&self) -> ImageRef<'_> {
    ImageRef::from(self)
  }

  /// Sets a new image for the layer.
  pub fn set_image(&self, image: Arc<Image>) {
    let (width, height) = image.dimensions();
    let mut borrow_mut = self.borrow_mut();
    let image_mut = borrow_mut.image_mut();
    image_mut.set_new_pixels(image.rgba(), width, height);
  }

  /// Creates a new Layer wrapper from an `Arc<Mutex<LayerInner>>`.
  pub(crate) fn from_inner(inner_layer: Arc<Mutex<LayerInner>>) -> Self {
    Layer { inner_layer }
  }

  /// Borrows the layer immutably.
  pub(crate) fn borrow(&self) -> std::sync::MutexGuard<'_, LayerInner> {
    self.inner_layer.lock().unwrap()
  }

  /// Borrows the layer mutably.
  pub(crate) fn borrow_mut(&self) -> std::sync::MutexGuard<'_, LayerInner> {
    self.inner_layer.lock().unwrap()
  }
}

/// Macro to generate immutable forwarding methods for `Layer` that return owned values
macro_rules! layer_method_imm_owned {
  ($(#[$meta:meta])* $name:ident() -> $ret:ty) => {
    $(#[$meta])*
    pub fn $name(&self) -> $ret {
      self.borrow().$name().to_string()
    }
  };
}

/// Macro to generate immutable forwarding methods for `Layer` that return scalars
macro_rules! layer_method_imm_scalar {
  ($(#[$meta:meta])* $name:ident() -> $ret:ty) => {
    $(#[$meta])*
    pub fn $name(&self) -> $ret {
      self.borrow().$name()
    }
  };
}

/// Macro to generate mutable forwarding methods for `Layer`
macro_rules! layer_method_mut {
  ($(#[$meta:meta])* $name:ident($($param:ident: $ty:ty),*)) => {
    $(#[$meta])*
    pub fn $name(&self, $($param: $ty),*) {
      self.borrow_mut().$name($($param),*);
    }
  };
}

impl Layer {
  layer_method_mut!(
    /// Sets the blend mode of the layer.
    set_blend_mode(blend_mode: fn(RGBA, RGBA) -> RGBA)
  );

  layer_method_mut!(
    /// Sets the opacity of the layer.
    set_opacity(opacity: f32)
  );

  /// Marks the parent canvas as needing recompose (forces a recompose on save).
  pub fn mark_dirty(&self) {
    self.borrow_mut().mark_dirty();
  }

  /// Returns a handler for applying transform operations to the layer.
  pub fn transform(&self) -> LayerTransform {
    LayerTransform::new(self.inner_layer.clone())
  }

  /// Returns the effects builder for queuing effects to be applied during rendering.
  pub fn effects(&self) -> LayerEffects {
    LayerEffects::new().with_layer(self.inner_layer.clone())
  }
  /// Sets all effects for the layer.
  pub fn set_effects(&self, effects: LayerEffects) {
    self.borrow_mut().set_effects(effects);
  }

  layer_method_mut!(
    /// Sets the visibility of the layer.
    set_visible(visible: bool)
  );

  layer_method_mut!(
    /// Sets the position of the layer.
    set_global_position(x: i32, y: i32)
  );

  layer_method_mut!(
    set_anchor(anchor: Option<Anchor>)
  );

  /// Sets the position of the layer relative to another `Layer`.
  pub fn set_relative_position(&self, x: i32, y: i32, layer: &Layer) {
    let other_layer = layer.borrow();
    self.borrow_mut().set_relative_position(x, y, &*other_layer);
    drop(other_layer);
  }

  layer_method_mut!(
    /// Anchors the layer to a specific position based on anchor point.
    anchor_to_canvas(anchor: Anchor)
  );

  layer_method_mut!(
    /// Sets the origin point within the layer for anchor positioning.
    /// The origin determines which point of the layer is aligned with the anchor.
    set_origin(origin: Origin)
  );

  layer_method_imm_owned!(
    /// Gets the name of the layer.
    name() -> String
  );

  layer_method_mut!(
    /// Sets the name of the layer.
    set_name(name: impl Into<String>)
  );

  layer_method_imm_scalar!(
    /// Gets the opacity of the layer.
    opacity() -> f32
  );

  layer_method_imm_scalar!(
    /// Gets the blend mode of the layer.
    blend_mode() -> fn(RGBA, RGBA) -> RGBA
  );

  layer_method_imm_scalar!(
    /// Gets whether the layer is visible.
    is_visible() -> bool
  );

  layer_method_imm_scalar!(
    /// Gets the position of the layer.
    position() -> (i32, i32)
  );

  /// Gets the anchor point of the layer.
  pub fn anchor(&self) -> Option<Anchor> {
    self.borrow().anchor()
  }

  /// Gets the current index of this layer in the canvas's layer stack.
  pub fn current_index(&self) -> Option<usize> {
    self.borrow().current_index()
  }

  /// Gets the adjustment type if this is an adjustment layer.
  pub fn adjustment_type(&self) -> Option<AdjustmentLayerType> {
    self.borrow().adjustment_type()
  }

  /// Gets the dimensions of the layer.
  pub fn dimensions<T>(&self) -> (T, T)
  where
    T: TryFrom<u64>,
    <T as TryFrom<u64>>::Error: std::fmt::Debug,
  {
    self.borrow().dimensions::<T>()
  }

  layer_method_mut!(
    /// Moves the layer up one position in the stack (increases its index by 1).
    /// Does nothing if the layer is already at the top.
    move_up()
  );

  layer_method_mut!(
    /// Moves the layer down one position in the stack (decreases its index by 1).
    /// Does nothing if the layer is already at the bottom.
    move_down()
  );

  layer_method_mut!(
    /// Moves the layer to the top of the stack.
    move_to_top()
  );

  layer_method_mut!(
    /// Moves the layer to the bottom of the stack.
    move_to_bottom()
  );

  layer_method_mut!(
    /// Moves the layer to the specified index in the stack.
    set_index(new_index: usize)
  );

  /// Duplicates the layer and returns a new `Layer` instance.
  pub fn duplicate(&self) -> Layer {
    self.borrow().duplicate()
  }

  layer_method_imm_owned!(
    /// Gets the UUID of the layer.
    id() -> String
  );

  // NOTE: No convenience `despeckle` here: prefer callers to use `image_mut` or
  // convert the `Layer` into a `MutexGuard` and operate on the `Image`.
}

impl Clone for Layer {
  fn clone(&self) -> Self {
    Layer {
      inner_layer: self.inner_layer.clone(),
    }
  }
}

/// Convert a `&mut Layer` into a `MutexGuard<'_, LayerInner>` so callers can
/// access the interior `Image` safely for as long as they need it.
impl<'a> From<&'a mut Layer> for MutexGuard<'a, LayerInner> {
  fn from(layer: &'a mut Layer) -> Self {
    layer.borrow_mut()
  }
}

/// Internal owner wrapper to keep track of the layer guard and satisfy the orphan
/// rules (we implement `core::GuardedOwner` for this local type so it can be
/// stored inside `ImageRef` trait object.
/// Internal owner wrapper to keep track of the layer guard and satisfy the orphan
/// rules (we implement `core::GuardedOwner` for this local type so it can be
/// stored inside `ImageRef` trait object).
///
/// The field is intentionally unused beyond being stored — we don't need to call
/// methods on the guard. The purpose is to *hold* the MutexGuard (keep the lock)
/// for as long as the `LayerGuardOwner` is alive, preventing the lock from being
/// dropped while the `ImageRef` exists. The underscore prefix avoids an "unused
/// field" lint/warning while making the intent clear.
struct LayerGuardOwner<'a> {
  _guard: MutexGuard<'a, LayerInner>,
}

impl<'a> GuardedOwner for LayerGuardOwner<'a> {}

/// Convert a `&mut Layer` into an `ImageRef` that owns the guard for as long as the ImageRef
/// is alive. This allows filters to take `impl Into<ImageRef>` and do `let mut image = p_image.into();`.
impl<'a> From<&'a mut Layer> for ImageRef<'a> {
  fn from(layer: &'a mut Layer) -> Self {
    // Acquire the guard from the layer (this keeps the mutex locked)
    let mut guard = layer.borrow_mut();
    // Get raw pointer to the image
    let ptr = guard.image_mut() as *mut Image;
    // Box the guard and erase the type via GuardedOwner trait object so ImageRef can own it
    let owner: Option<Box<dyn GuardedOwner + 'a>> = Some(Box::new(LayerGuardOwner { _guard: guard }));
    ImageRef::new(ptr, owner)
  }
}

/// Convert an immutable `&Layer` into an `ImageRef` that owns the mutex guard
/// for as long as the `ImageRef` exists. This allows callers to borrow the
/// `Image` by reference without cloning the underlying data while the lock is
/// held.
impl<'a> From<&'a Layer> for ImageRef<'a> {
  fn from(layer: &'a Layer) -> Self {
    let guard = layer.borrow();
    // Obtain a raw pointer to the image; convert from const to mut for the
    // ImageRef which expects *mut — the guard/owner enforces safety at runtime.
    let ptr = guard.image() as *const Image as *mut Image;
    let owner: Option<Box<dyn GuardedOwner + 'a>> = Some(Box::new(LayerGuardOwner { _guard: guard }));
    ImageRef::new(ptr, owner)
  }
}
