# Layer

Public wrapper around an image with stacking, blending, positioning, and effects.

## Summary
- Create: `Layer::new(name, Arc<Image>)` (rarely needed directly; typically created via `Canvas::add_layer_*`).
- Identity: `id()`, `name()`, `set_name()`.
- Visibility/Blend: `set_visible`, `is_visible`, `set_opacity`, `opacity`, `set_blend_mode`, `blend_mode`.
- Positioning: `set_global_position`, `position`, `set_relative_position`, `anchor_to_canvas`, `set_origin`.
- Size: `dimensions<T>()`.
- Ordering: `move_up`, `move_down`, `move_to_top`, `move_to_bottom`.
- Copy: `duplicate()`.
- Transform: `transform()` → `LayerTransform`.
- Effects: `effects()` builder; `set_effects()` to commit.

## Example
```rust
use abra::{Canvas, Anchor};
use abra::canvas::NewLayerOptions;

let canvas = Canvas::new_blank("Doc", 800, 600)
  .add_layer_from_path("Photo", "assets/photo.jpg", Some(NewLayerOptions::new()))
  .add_layer_from_path("Mark", "assets/mark.png", None);

let photo = canvas.get_layer_by_name("Photo").unwrap();
photo.set_opacity(0.9);
photo.anchor_to_canvas(Anchor::Center);
photo.transform().resize_width(600, None);

let mark = canvas.get_layer_by_name("Mark").unwrap();
mark.set_relative_position(20, -20, &photo);
```

## API Reference

### new
Creates a new layer from an `Arc<Image>` (usually created via `Canvas::add_layer_*`).
```rust
use std::sync::Arc;
use abra::{Canvas, Image};
let img = Arc::new(Image::new_from_path("assets/pic.png"));
let layer = abra::canvas::Layer::new("Pic", img);
```

### set_blend_mode / blend_mode
Sets or reads the blend mode function used when compositing.
```rust
use abra::combine::blend;
layer.set_blend_mode(blend::normal);
let current = layer.blend_mode();
```

### set_opacity / opacity
Sets or reads opacity (0.0–1.0).
```rust
layer.set_opacity(0.75);
let o = layer.opacity();
```

### set_visible / is_visible
Toggles visibility or queries it.
```rust
layer.set_visible(true);
assert!(layer.is_visible());
```

### set_global_position / position
Places the layer at absolute coordinates on the canvas.
```rust
layer.set_global_position(120, 40);
let (x, y) = layer.position();
```

### set_relative_position
Positions this layer relative to another layer’s position.
```rust
let base = canvas.get_layer_by_name("Photo").unwrap();
layer.set_relative_position(20, -10, &base);
```

### anchor_to_canvas
Stores an anchor to position this layer during composition.
```rust
use abra::Anchor;
layer.anchor_to_canvas(Anchor::TopRight);
```

### set_origin
Sets the in-layer reference point used for anchor placement.
```rust
use abra::canvas::Origin;
layer.set_origin(Origin::BottomLeft);
```

### name / set_name / id
Gets/sets the human-readable name and reads the stable UUID.
```rust
layer.set_name("Foreground");
let n = layer.name();
let id = layer.id();
```

### dimensions
Reads the current layer dimensions.
```rust
let (w, h): (u32, u32) = layer.dimensions();
```

### move_up / move_down / move_to_top / move_to_bottom
Reorders the layer within the canvas stack.
```rust
layer.move_up();
layer.move_to_top();
```

### duplicate
Creates a copy of this layer in the same canvas and returns it.
```rust
let copy = layer.duplicate();
```

### transform
Returns a `LayerTransform` to mutate the underlying image.
```rust
layer.transform()
  .resize_width(640, None)
  .crop(0, 0, 640, 360);
```

### effects / set_effects
Queues or sets effects to be applied at render time.
```rust
use abra::canvas::effects::{LayerEffects, DropShadow};
layer.effects().with_drop_shadow(DropShadow::new().with_distance(8.0));
// or
layer.set_effects(LayerEffects::new());
```
