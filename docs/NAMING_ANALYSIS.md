# Naming Convention Analysis - Abra Library

## Executive Summary
This document analyzes all public APIs across the Abra library to identify naming inconsistencies, particularly in the Canvas/Layer system.

---

## 1. Core Naming Patterns

### 1.1 Getter Methods
The library uses **multiple inconsistent patterns** for getters:

**Pattern A: Prefix-less (Property-style)**
- `LayerHandle::name()` - Returns `String`
- `LayerHandle::opacity()` - Returns `f32`
- `LayerHandle::blend_mode()` - Returns `fn(RGBA, RGBA) -> RGBA`
- `LayerHandle::is_visible()` - Returns `bool`
- `LayerHandle::position()` - Returns `(i32, i32)`
- `LayerHandle::dimensions<T>()` - Returns `(T, T)`
- `Canvas::dimensions()` - Returns `(u32, u32)`
- `Color::rgb()` - Returns `(u8, u8, u8)`
- `Color::rgba()` - Returns `(u8, u8, u8, u8)`
- `Color::hsl()` - Returns `(f32, f32, f32)`
- `Image::dimensions<T>()` - Returns `(T, T)`
- `Point::x()` - Returns `i32`
- `Point::y()` - Returns `i32`

**Pattern B: `get_` prefix (CORRECT for collection access)**
- `Canvas::get_layer_opacity(index)` - Returns `Option<f32>` ✅ Correct (collection access getter)
- `Canvas::get_layer_position(index)` - Returns `Option<(i32, i32)>` ✅ Correct (collection access getter)
- `Canvas::get_layer_visible(index)` - Returns `Option<bool>` ✅ Correct (collection access getter)
- `Canvas::get_layer_name(index)` - Returns `Option<String>` ✅ Correct (collection access getter)
- `Canvas::get_layer_blend_mode(index)` - Returns `Option<fn(RGBA, RGBA) -> RGBA>` ✅ Correct (collection access getter)
- `Canvas::get_layer_order()` - Returns `Vec<String>` ✅ Correct (collection access getter)
- `Canvas::get_layer(index)` - Returns `Option<Rc<RefCell<Layer>>>` ⚠️ Should be `get_layer_handle()` for consistency
- `RcLayersHandler::get_layer_opacity(index)` - Returns `Option<f32>` ✅ Correct (collection access getter)
- `RcLayersHandler::get_layer_position(index)` - Returns `Option<(i32, i32)>` ✅ Correct (collection access getter)
- `RcLayersHandler::get_layer_visible(index)` - Returns `Option<bool>` ✅ Correct (collection access getter)
- `RcLayersHandler::get_layer_name(index)` - Returns `Option<String>` ✅ Correct (collection access getter)
- `Path::get_points()` - Returns `&Vec<Point>` ✅ Correct (collection access)
- `Path::get_point_at(index)` - Returns `Point` ✅ Correct (collection access getter)
- `Color::contrast_ratio(other)` - Returns `f32` ✅ Good (no redundant get_)
- `Color::luminance()` - Returns `f32` ✅ Good (no redundant get_)

**Pattern C: `as_` prefix**
- `Image::as_ref()` - Removed/redundant — prefer dereferencing or `Arc::as_ref()` where applicable
- `Image::as_ref_mut()` - Removed/redundant — prefer direct mutable reference

### 1.2 Setter Methods
Setters are highly consistent with `set_` prefix:

- `LayerHandle::set_opacity(opacity)`
- `LayerHandle::set_blend_mode(blend_mode)`
- `LayerHandle::set_visible(visible)`
- `LayerHandle::set_global_position(x, y)`
- `LayerHandle::set_relative_position(x, y, layer)`
- `LayerHandle::set_name(name)`
- `Layer::set_opacity(opacity)`
- `Layer::set_blend_mode(blend_mode)`
- `Layer::set_visible(visible)`
- `Layer::set_global_position(x, y)`
- `Layer::set_relative_position(x, y, layer)`
- `Layer::set_name(name)`
- `Canvas::set_layer_opacity(index, opacity)`
- `Canvas::set_layer_blend_mode(index, blend_mode)`
- `Canvas::set_layer_visible(index, visible)`
- `Canvas::set_layer_global_position(index, x, y)` - Will be renamed from `set_layer_position()` for clarity
- `Canvas::set_layer_name(index, name)`
- `RcLayersHandler::set_layer_opacity(index, opacity)`
- `RcLayersHandler::set_layer_blend_mode(index, blend_mode)`
- `RcLayersHandler::set_layer_visible(index, visible)`
- `RcLayersHandler::set_layer_position(index, x, y)` - Will be renamed to `set_layer_global_position()` for clarity
- `Image::set_rgba(data)`
- `Image::set_rgb(data)`
- `Image::set_new_pixels(data, width, height)`
- `Image::set_pixel(x, y, pixel)`
// NOTE: `Image::set_colors` and `Image::set_channel` removed — prefer `set_rgba_owned/set_rgba` and `mut_channel`

---

## 2. Position Method Analysis

### Current Naming (Inconsistent)

**On Layer:**
- `set_global_position(x, y)` - Set absolute position
- `set_relative_position(x, y, layer)` - Set position relative to another layer
- `anchor_to_canvas(anchor)` - Anchor to canvas corner
- `anchor_to_layer(anchor, layer)` - Anchor to another layer corner
- `position()` - Get position (no prefix!)

**On Canvas/Handler:**
- `set_layer_position(index, x, y)` - Sets global (absolute) position (calls `set_global_position` on layer)
- `get_layer_position(index)` - Get position
- `anchor_layer_to_canvas(index, anchor)` - Anchor to canvas
- `move_layer(from_index, to_index)` - Move in layer stack
- `move_layer_to_top(index)` - Move to top of stack
- `move_layer_to_bottom(index)` - Move to bottom of stack

### Issues
1. **Inconsistent naming**: `set_layer_position()` should be `set_layer_global_position()` for clarity
2. **Missing method**: `set_layer_position_offset()` needed for relative positioning
3. **Position getter semantic**: `get_layer_position()` is consistent (returns current position)

---

## 3. Layer Access Pattern Analysis

### Multiple API Styles

**Style 1: Index-based on Canvas**
```rust
project.set_layer_opacity(0, 0.5);
project.get_layer_opacity(0);
project.move_layer(0, 1);
```

**Style 2: Index-based on Handler**
```rust
project.with_layers(|layers| {
    layers.set_layer_opacity(0, 0.5);
    layers.get_layer_opacity(0);
    layers.move_to(0, 1);
});
```

**Style 3: Handle-based (direct)**
```rust
if let Some(layer) = project.get_layer_by_index(0) {
    layer.set_opacity(0.5);
    layer.opacity();
}
```

### Issues
1. **Different method names for same operation**: `move_layer()` vs `move_to()`
2. **Handler methods don't consistently match Canvas methods**
3. **Three different APIs for same operations**

---

## 4. Recommended Naming Standard

### Phase 1: Standardize Position Methods (Highest Priority)

On **Layer** and **LayerHandle**:
```rust
// Getters - Rename to be consistent
pub fn position(&self) -> (i32, i32)        // ✅ Already correct

// Setters - Standardize naming
pub fn set_position(&self, x: i32, y: i32)                 // ✅ Already correct (same as set_global_position)
pub fn set_position_offset(&self, x: i32, y: i32, layer: &Layer) // ✅ Already correct (same as set_relative_position)
pub fn anchor_to_canvas(&self, anchor: Anchor)             // ✅ Keep as-is (clear semantics)
pub fn anchor_to_layer(&self, anchor: Anchor, layer: &Layer)     // ✅ Keep as-is (clear semantics)
```

On **Canvas**:
```rust
// Getters - Add consistency
pub fn get_layer_position(&self, index: usize) -> Option<(i32, i32)>  // ✅ Already correct

// Setters - Clarify semantics
pub fn set_layer_global_position(&self, index: usize, x: i32, y: i32)  // RENAME from set_layer_position (currently ambiguous)
pub fn set_layer_position_offset(&self, index: usize, x: i32, y: i32, target_index: usize) // NEW - for relative positioning
pub fn anchor_layer_to_canvas(&self, index: usize, anchor: Anchor)     // ✅ Keep as-is (clear semantics)

// Movement - Keep stack operations separate but consistent (move_ is the action verb)
pub fn move_layer(&self, from_index: usize, to_index: usize)  // ✅ Keep as is
pub fn move_layer_to_top(&self, index: usize)                 // ✅ Keep as is
pub fn move_layer_to_bottom(&self, index: usize)              // ✅ Keep as is
```

### Phase 2: Standardize Handler Method Names

Change **RcLayersHandler** methods for consistency:
```rust
// Current inconsistency: Canvas uses move_layer, Handler uses move_to
// Standardize to Canvas naming:
pub fn move_layer(&self, from_index: usize, to_index: usize)  // RENAME from move_to
pub fn move_layer_to_top(&self, index: usize)                 // RENAME from move_to_top
pub fn move_layer_to_bottom(&self, index: usize)              // RENAME from move_to_bottom
```

### Phase 3: Finalize Getter Pattern

Establish rule: **Use `get_` prefix ONLY when accessing items from collections/arrays, with context-dependent naming**

Standard naming:
- **On Layer/LayerHandle** (operating on self): NO prefix on property names
  - `layer.name()`, `layer.opacity()`, `layer.position()`, `layer.set_opacity()`
  - User already knows it's the layer's property

- **On Canvas/Handler** (accessing from collection with index): WITH `get_` prefix + `layer_` for disambiguation
  - `canvas.get_layer_opacity(index)`, `canvas.set_layer_name(index, value)`
  - Needs `layer_` prefix because you're disambiguating which object's property you're accessing

- **Setters (all)**: Always `set_` prefix with appropriate context
  - Layer: `set_opacity()`
  - Canvas: `set_layer_opacity(index, value)`---

## 5. Complete Naming Reference

### LayerHandle (Direct Access)
```rust
// Getters - NO "layer" prefix needed, already operating on Layer
pub fn name(&self) -> String
pub fn opacity(&self) -> f32
pub fn blend_mode(&self) -> fn(RGBA, RGBA) -> RGBA
pub fn is_visible(&self) -> bool
pub fn position(&self) -> (i32, i32)
pub fn dimensions<T>(&self) -> (T, T)

// Setters - NO "layer" prefix needed, already operating on Layer
pub fn set_name(&self, name: &str)
pub fn set_opacity(&self, opacity: f32)
pub fn set_blend_mode(&self, blend_mode: fn(RGBA, RGBA) -> RGBA)
pub fn set_visible(&self, visible: bool)
pub fn set_global_position(&self, x: i32, y: i32)          // ✅ Keep as-is (clear semantics)
pub fn set_relative_position(&self, x: i32, y: i32, layer: &Layer) // ✅ Keep as-is (clear semantics)
pub fn anchor_to_canvas(&self, anchor: Anchor)             // ✅ Keep as-is (clear semantics)
pub fn anchor_to_layer(&self, anchor: Anchor, layer: &Layer) // ✅ Keep as-is (clear semantics)
```

### Canvas (Index-based - Collection Access)
```rust
// Layer access - Return LayerHandle or raw types (WITH get_ prefix - accessing FROM collection)
pub fn get_layer(&self, index: usize) -> Option<Rc<RefCell<Layer>>>  // RENAME to get_layer_handle() for consistency
pub fn get_layer_by_index(&self, index: usize) -> Option<LayerHandle>
pub fn get_layer_by_name(&self, name: &str) -> Option<LayerHandle>
pub fn get_layer_count(&self) -> usize  // RENAME from layer_count()

// Getters from collection (WITH get_ prefix - accessing FROM the array, needs "layer_" to disambiguate)
pub fn get_layer_opacity(&self, index: usize) -> Option<f32>
pub fn get_layer_blend_mode(&self, index: usize) -> Option<fn(RGBA, RGBA) -> RGBA>
pub fn get_layer_visible(&self, index: usize) -> Option<bool>
pub fn get_layer_name(&self, index: usize) -> Option<String>
pub fn get_layer_position(&self, index: usize) -> Option<(i32, i32)>
pub fn get_layer_order(&self) -> Vec<String>

// Setters to collection (WITH set_ + "layer_" prefix - setting IN the array)
pub fn set_layer_name(&self, index: usize, name: &str)
pub fn set_layer_opacity(&self, index: usize, opacity: f32)
pub fn set_layer_blend_mode(&self, index: usize, blend_mode: fn(RGBA, RGBA) -> RGBA)
pub fn set_layer_visible(&self, index: usize, visible: bool)
pub fn set_layer_global_position(&self, index: usize, x: i32, y: i32)  // RENAME from set_layer_position (clarifies it's global)
pub fn set_layer_position_offset(&self, index: usize, x: i32, y: i32, target_index: usize) // NEW (for relative positioning)
pub fn anchor_layer_to_canvas(&self, index: usize, anchor: Anchor)     // ✅ Keep as-is (clear semantics)

// Stack operations (move_ is the action verb)
pub fn move_layer(&self, from_index: usize, to_index: usize)
pub fn move_layer_to_top(&self, index: usize)
pub fn move_layer_to_bottom(&self, index: usize)
```

### RcLayersHandler (Index-based - Collection Access)
```rust
// Getters from collection (WITH get_ prefix - accessing FROM the array, needs "layer_" to disambiguate)
pub fn get_layer_opacity(&self, index: usize) -> Option<f32>
pub fn get_layer_blend_mode(&self, index: usize) -> Option<fn(RGBA, RGBA) -> RGBA>
pub fn get_layer_visible(&self, index: usize) -> Option<bool>
pub fn get_layer_name(&self, index: usize) -> Option<String>
pub fn get_layer_position(&self, index: usize) -> Option<(i32, i32)>

// Setters to collection (WITH set_ + "layer_" prefix - setting IN the array)
pub fn set_layer_name(&self, index: usize, name: &str)
pub fn set_layer_opacity(&self, index: usize, opacity: f32)
pub fn set_layer_blend_mode(&self, index: usize, blend_mode: fn(RGBA, RGBA) -> RGBA)
pub fn set_layer_visible(&self, index: usize, visible: bool)
pub fn set_layer_global_position(&self, index: usize, x: i32, y: i32)  // RENAME from set_layer_position

// Stack operations - Rename for consistency with Canvas
pub fn move_layer(&self, from_index: usize, to_index: usize)        // RENAME from move_to
pub fn move_layer_to_top(&self, index: usize)                       // RENAME from move_to_top
pub fn move_layer_to_bottom(&self, index: usize)                    // RENAME from move_to_bottom
```

---

## 6. Migration Plan

### Step 1: Update LayerHandle (No renames needed)
- Keep: `set_global_position()` (clear semantics)
- Keep: `set_relative_position()` (clear semantics)
- Keep: `anchor_to_canvas()` (clear semantics - clarifies anchor is to canvas)
- Keep: `anchor_to_layer()` (clear semantics - clarifies anchor is to layer)

### Step 2: Update Layer (No renames needed)
- Keep: `set_global_position()` (clear semantics)
- Keep: `set_relative_position()` (clear semantics)
- Keep: `anchor_to_canvas()` (clear semantics - clarifies anchor is to canvas)
- Keep: `anchor_to_layer()` (clear semantics - clarifies anchor is to layer)

### Step 3: Update Canvas (Rename accessor methods only)
NO changes to `get_layer_*` methods - they're correct as-is (they access from the collection)
- Keep: `get_layer_opacity()`, `get_layer_position()`, `get_layer_visible()`, `get_layer_name()`, `get_layer_blend_mode()`, `get_layer_order()`
- Keep: `anchor_layer_to_canvas()` (clear semantics - clarifies anchor is to canvas)
- Rename: `get_layer()` → `get_layer_handle()` (for consistency - returns LayerHandle-like object)
- Rename: `set_layer_position()` → `set_layer_global_position()` (clarifies it sets global position)
- Rename: `layer_count()` → `get_layer_count()` (accessor from collection)
- Add: `set_layer_position_offset()` (for relative positioning)
- Already correct: `get_layer_by_index()`, `get_layer_by_name()`, `move_layer()`, `move_layer_to_top()`, `move_layer_to_bottom()`

### Step 4: Update RcLayersHandler (Rename move methods + position method)
NO changes to `get_layer_*` methods - they're correct as-is (they access from the collection)
- Keep: `get_layer_opacity()`, `get_layer_position()`, `get_layer_visible()`, `get_layer_name()`
- Rename: `set_layer_position()` → `set_layer_global_position()` (clarifies it sets global position)
- Rename: `move_to()` → `move_layer()`
- Rename: `move_to_top()` → `move_layer_to_top()`
- Rename: `move_to_bottom()` → `move_layer_to_bottom()`

### Step 5: Update test files

---

## 7. Additional Library-wide Observations

### Consistent Patterns
- ✅ Setter methods are **highly consistent** with `set_` prefix across all types
- ✅ Constructor methods use `new()` consistently
- ✅ Factory methods use `from_*` pattern (e.g., `Color::from_rgb()`, `Image::from_path()`)
- ✅ Conditional constructors use `new_*` pattern (e.g., `Canvas::new_blank()`)

### Areas to Monitor
- `get_at()` / `get_layer()` - Could be inconsistent with new API
- `add_layer_*()` methods return different types across classes
- `dimensions()` sometimes generic `<T>`, sometimes fixed `u32`

---

## Summary Table

| Issue                      | Current                    | Recommended                   | Type                           | Impact                 |
| -------------------------- | -------------------------- | ----------------------------- | ------------------------------ | ---------------------- |
| Position setter on Layer   | `set_global_position()`    | Keep as-is ✅                  | Clear semantics                | ✅ No change            |
| Relative position on Layer | `set_relative_position()`  | Keep as-is ✅                  | Clear semantics                | ✅ No change            |
| Anchor to canvas on Layer  | `anchor_to_canvas()`       | Keep as-is ✅                  | Clear semantics                | ✅ No change            |
| Anchor to layer on Layer   | `anchor_to_layer()`        | Keep as-is ✅                  | Clear semantics                | ✅ No change            |
| Anchor on Canvas           | `anchor_layer_to_canvas()` | Keep as-is ✅                  | Clear semantics                | ✅ No change            |
| Get layer opacity          | `get_layer_opacity()`      | Keep as-is ✅                  | Collection access (correct)    | ✅ No change            |
| Get layer position         | `get_layer_position()`     | Keep as-is ✅                  | Collection access (correct)    | ✅ No change            |
| Get layer visible          | `get_layer_visible()`      | Keep as-is ✅                  | Collection access (correct)    | ✅ No change            |
| Get layer name             | `get_layer_name()`         | Keep as-is ✅                  | Collection access (correct)    | ✅ No change            |
| Get layer blend mode       | `get_layer_blend_mode()`   | Keep as-is ✅                  | Collection access (correct)    | ✅ No change            |
| Get layer order            | `get_layer_order()`        | Keep as-is ✅                  | Collection access (correct)    | ✅ No change            |
| Get layer by index         | `layer_by_index()`         | `get_layer_by_index()` ✅      | Collection access (correct)    | ✅ Rename               |
| Get layer by name          | `layer_by_name()`          | `get_layer_by_name()` ✅       | Collection access (correct)    | ✅ Rename               |
| Get layer (raw Rc)         | `get_layer()`              | `get_layer_handle()`          | Consistency with accessors     | ⭐ Medium - Consistency |
| Set layer position         | `set_layer_position()`     | `set_layer_global_position()` | Clear semantics (not relative) | ⭐⭐ High - Clarity      |
| Get layer count            | `layer_count()`            | `get_layer_count()`           | Collection access pattern      | ⭐⭐ High - Consistency  |
| Move in Handler            | `move_to()`                | `move_layer()`                | Consistency with Canvas        | ⭐⭐ High - Consistency  |
| Move to top in Handler     | `move_to_top()`            | `move_layer_to_top()`         | Consistency with Canvas        | ⭐⭐ High - Consistency  |
| Move to bottom in Handler  | `move_to_bottom()`         | `move_layer_to_bottom()`      | Consistency with Canvas        | ⭐⭐ High - Consistency  |
