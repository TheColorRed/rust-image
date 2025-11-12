//! The Layer public API struct.

use std::sync::Arc;
use std::sync::Mutex;

use crate::canvas::{LayerEffects, layer_inner::LayerInner};
use crate::combine::blend::RGBA;

pub use super::anchor::Anchor;
pub use super::layer_transform::LayerTransform;
pub use super::origin::Origin;

/// A layer in a project.
/// This is the public API struct that wraps `Arc<Mutex<LayerInner>>`.
#[derive(Debug)]
pub struct Layer {
  /// Reference to the inner layer.
  inner_layer: Arc<Mutex<LayerInner>>,
}

impl Layer {
  /// Creates a new layer with the given name and image.
  pub fn new(name: &str, image: Arc<crate::image::Image>) -> Self {
    Layer {
      inner_layer: Arc::new(Mutex::new(LayerInner::new(name, image))),
    }
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
  // Convenience methods that forward directly to layer without explicit borrows

  layer_method_mut!(
    /// Sets the blend mode of the layer.
    set_blend_mode(blend_mode: fn(RGBA, RGBA) -> RGBA)
  );

  layer_method_mut!(
    /// Sets the opacity of the layer.
    set_opacity(opacity: f32)
  );

  /// Returns a handler for applying transform operations to the layer.
  pub fn transform(&self) -> LayerTransform {
    LayerTransform::new(self.inner_layer.clone())
  }

  /// Returns the effects builder for queuing effects to be applied during rendering.
  pub fn effects(&self) -> LayerEffects {
    LayerEffects::new().with_layer(self.inner_layer.clone())
  }

  layer_method_mut!(
    /// Sets the visibility of the layer.
    set_visible(visible: bool)
  );

  layer_method_mut!(
    /// Sets the position of the layer.
    set_global_position(x: i32, y: i32)
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
    set_name(name: &str)
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

  /// Gets the dimensions of the layer.
  pub fn dimensions<T>(&self) -> (T, T)
  where
    T: TryFrom<u32>,
    <T as TryFrom<u32>>::Error: std::fmt::Debug,
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

  /// Duplicates the layer and returns a new `Layer` instance.
  pub fn duplicate(&self) -> Layer {
    self.borrow().duplicate()
  }

  layer_method_imm_owned!(
    /// Gets the UUID of the layer.
    id() -> String
  );
}

impl Clone for Layer {
  fn clone(&self) -> Self {
    Layer {
      inner_layer: self.inner_layer.clone(),
    }
  }
}
