# Canvas/Layer API Review

## Overview
The Canvas/Layer API is designed for image processing with support for layered composition. This document tracks remaining improvements needed for optimal usability.

---

## ✅ Completed Improvements

The following issues have been successfully resolved:

1. **Fluent API for Method Chaining** - Canvas methods now return `Self` enabling `.add_layer_from_path().add_layer_from_path()` chains
2. **Handler Convenience Methods** - `RcLayersHandler` provides `set_layer_opacity()`, `set_layer_blend_mode()`, `set_layer_visible()`, `move_to()`, etc.
3. **Field Encapsulation** - All Layer fields are now private with proper getter/setter methods that trigger automatic recomposition
4. **Layer Lookup Methods** - Added `get_layer_by_index()` and `get_layer_by_name()` for easy layer discovery
5. **Canvas Convenience Methods** - Added 15+ methods to Canvas to eliminate `.borrow()` boilerplate:
   - Setters: `set_layer_opacity()`, `set_layer_blend_mode()`, `set_layer_visible()`, `set_layer_position()`, `set_layer_name()`, `anchor_layer_to_canvas()`, `move_layer()`, `move_layer_to_top()`, `move_layer_to_bottom()`
   - Getters: `get_layer_opacity()`, `get_layer_position()`, `get_layer_visible()`, `get_layer_name()`, `get_layer_blend_mode()`
6. **LayerHandle Wrapper** - New `LayerHandle` type wraps `Rc<RefCell<Layer>>` with direct method forwarding, eliminating `.borrow()` calls entirely:
   - Users can now call `layer.set_opacity(0.8)` directly without `.borrow_mut()`
   - Users can now call `layer.name()` directly without `.borrow()`

---

## Remaining Issues & Recommendations

### 🟡 **MAJOR: NewLayerOptions is Incomplete**

**Problem:** `NewLayerOptions` only supports `anchor`. When adding a layer, you often want to set:
- Position
- Opacity
- Blend mode
- Visibility

Currently you must do:
```rust
let layer = canvas.add_layer_from_path("Top", TOP_IMAGE, None);
layer.set_opacity(0.5);
layer.set_blend_mode(blend::multiply);
```

**Recommendation:** Expand `NewLayerOptions`:

```rust
pub struct NewLayerOptions {
    pub anchor: Option<Anchor>,
    pub position: Option<(i32, i32)>,
    pub opacity: Option<f32>,
    pub blend_mode: Option<fn(RGBA, RGBA) -> RGBA>,
    pub visible: Option<bool>,
}
```

Or provide a builder pattern:

```rust
NewLayerOptionsBuilder::new()
    .anchor(Anchor::Center)
    .opacity(0.5)
    .blend_mode(blend::multiply)
    .build()
```

---

### 🟡 **MAJOR: Inconsistent Method Naming**

**Problem:** Naming conventions are inconsistent for position-related methods:

```rust
// Layer position methods (no consistent naming):
pub fn set_global_position(&mut self, x: i32, y: i32)
pub fn set_relative_position(&mut self, x: i32, y: i32, layer: &Layer)
pub fn anchor_to_canvas(&mut self, anchor: Anchor)
pub fn anchor_to_layer(&mut self, anchor: Anchor, layer: &Layer)
pub fn position(&self) -> (i32, i32)  // Inconsistent - should be get_position or stay as is
```

**Recommendation:**
- Standardize on either `position()` or `get_position()` for consistency with other getters
- Consider renaming `set_relative_position()` to something clearer like `set_position_relative()` or `set_position_offset()`
- Clarify anchor methods: `anchor_to_canvas()` vs `anchor_to_layer()` naming

---

### 🟡 **MEDIUM: Canvas Initialization Options**

**Problem:** Three different constructors with subtle differences:

```rust
pub fn new(name: &str) -> Self
pub fn new_blank(name: &str, width: u32, height: u32) -> Self
pub fn new_from_path(name: &str, path: &str, options: Option<NewLayerOptions>) -> Self
```

`new_from_path()` adds the image as the first layer, but `new()` and `new_blank()` don't. This is not obvious from the names.

**Recommendation:** Consider renaming for clarity:
```rust
pub fn new(name: &str) -> Self  // Empty canvas
pub fn new_with_size(name: &str, width: u32, height: u32) -> Self
pub fn new_from_image(name: &str, path: &str, options: Option<NewLayerOptions>) -> Self
```

Or use a builder:
```rust
Canvas::builder("My Project")
    .size(800, 600)
    .build()
```

---

### 🟡 **MEDIUM: Missing High-Level Operations**

**Problem:** Common operations are not provided:

```rust
// No way to easily:
// - Flatten/merge all layers into one
// - Get all layers as a vector
// - Iterate over layers efficiently
```

**Implemented:**
- ✅ `get_layer_by_index(index)` - Get layer at index
- ✅ `get_layer_by_name(name)` - Get layer by name

**Still TODO:**
- `flatten()` - Merge all layers into one image
- `layers()` - Get all layers as a vector
- Implement Iterator trait for layers

---

### 🟢 **MINOR: Documentation**

**Problem:** Missing examples in doc comments and explanation of the `LayerHandle` pattern.

**Recommendation:**
- Add doctests to LayerHandle methods showing usage without `.borrow()`
- Explain the `LayerHandle` pattern and when to use it
- Show common workflows (adding layers, modifying properties, saving)

---

## Summary of Remaining Work

| Priority | Issue                   | Status |
| -------- | ----------------------- | ------ |
| 🟡 MAJOR  | Limited NewLayerOptions | TODO   |
| 🟡 MAJOR  | Inconsistent naming     | TODO   |
| 🟡 MEDIUM | Constructor clarity     | TODO   |
| 🟡 MEDIUM | Missing operations      | TODO   |
| 🟢 MINOR  | Documentation           | TODO   |

---

## Example: Simplified API With Current Improvements

```rust
// Simple workflow - much cleaner now!
let project = Canvas::new("My Project")
    .add_layer_from_path("Background", "bg.png", None)
    .add_layer_from_path("Overlay", "overlay.png", None);

// Get layer by name without .borrow()!
if let Some(layer) = project.get_layer_by_name("Background") {
    layer.set_opacity(0.8);
    layer.set_blend_mode(blend::multiply);
}

// Or use Canvas convenience methods
project.set_layer_opacity(0, 0.8);
project.set_layer_blend_mode(1, blend::multiply);

// Save
project.save("output.png", None);
```

Much cleaner! ✨
