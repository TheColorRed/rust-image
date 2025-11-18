# Canvas

High-level container for composing layers and child canvases. Provides creation, composition, transforms, and IO.

## Summary
- Purpose: manage layers, child canvases, and export a final image.
- Create: `Canvas::new`, `Canvas::new_blank`, `Canvas::new_from_path`.
- Compose: `add_layer_from_path`, `add_layer_from_image`, `add_canvas`.
- Layout: `set_position`, `position`, `set_rotation`, `rotation`, `set_origin`, `origin`, `anchor_to_canvas`.
- Output: `save`, `as_image`, `flatten`.
- Transform: `transform()` → `CanvasTransform`.

## Quick Start
```rust
use abra::{Canvas, Anchor};
use abra::canvas::{NewLayerOptions, LayerSize};

let canvas = Canvas::new("My Project")
  .add_layer_from_path("Background", "assets/bg.png", Some(NewLayerOptions::new()
    .with_size(LayerSize::Cover(None))
    .with_anchor(Anchor::Center)))
  .add_layer_from_path("Logo", "assets/logo.png", None);

canvas.save("out/result.png", None);
```

## Nesting Canvases
```rust
use abra::canvas::{AddCanvasOptions};

let parent = Canvas::new_blank("Parent", 1024, 768);
let child = Canvas::new_from_path("Child", "assets/photo.jpg", None);
parent.add_canvas(child, Some(AddCanvasOptions::new().with_rotation(15.0)));
parent.save("out/nested.png", None);
```

## Transforms
```rust
let mut t = canvas.transform();
t.resize(800, 600, None).rotate(5.0, None);
```

## Notes
- Composition is automatic; `save` and `as_image` trigger recomposition if needed.
- The first added layer or child canvas can establish canvas size when initially zero.

## API Reference

### new
Creates an empty canvas (0×0) with a project name.
```rust
use abra::Canvas;
let canvas = Canvas::new("Project");
```

### new_blank
Creates a blank canvas with a fixed size.
```rust
use abra::Canvas;
let canvas = Canvas::new_blank("Poster", 1920, 1080);
```

### new_from_path
Creates a canvas sized to an image loaded from disk.
```rust
use abra::canvas::NewLayerOptions;
let canvas = Canvas::new_from_path("From Image", "assets/bg.png", Some(NewLayerOptions::new()));
```

### save
Recomposes (if needed) and writes the flattened result to disk.
```rust
canvas.save("out/result.png", None);
```

### as_image
Returns a flattened `Image` of the current composition.
```rust
let img = canvas.as_image();
// img.save("out.png", None);
```

### flatten
Merges all layers into a single layer within the same canvas.
```rust
let canvas = canvas.flatten();
canvas.save("out/flattened.png", None);
```



### dimensions
Gets canvas size. Specify the return type as needed.
```rust
let (w, h): (u32, u32) = canvas.dimensions();
```

### position / set_position
Gets or sets the canvas position relative to its parent canvas.
```rust
canvas.set_position(100, 50);
let (x, y) = canvas.position();
```

### set_rotation / rotation
Sets or reads rotation in degrees relative to the parent.
```rust
let mut c = canvas; // if you need &mut for set_rotation
c.set_rotation(Some(15.0));
assert_eq!(c.rotation(), Some(15.0));
```

### set_origin / origin
Sets or reads the internal origin used for anchor placement.
```rust
use abra::canvas::Origin;
canvas.set_origin(Origin::TopLeft);
let o = canvas.origin();
```

### add_layer_from_path
Adds a new layer by loading an image from disk. Returns `Self` for chaining.
```rust
use abra::{Canvas, Anchor};
use abra::canvas::{NewLayerOptions, LayerSize};
let canvas = Canvas::new("Chain")
  .add_layer_from_path("BG", "assets/bg.jpg", Some(NewLayerOptions::new().with_size(LayerSize::Cover(None)).with_anchor(Anchor::Center)))
  .add_layer_from_path("Logo", "assets/logo.png", None);
```

### add_layer_from_image
Adds a layer from an existing `Arc<Image>`.
```rust
use std::sync::Arc;
use abra::{Canvas};
use abra::Image;
let img = Arc::new(Image::new_from_path("assets/pic.png"));
let canvas = Canvas::new("From Arc").add_layer_from_image("Pic", img, None);
```

### add_canvas
Nests a child canvas into this canvas with options.
```rust
use abra::canvas::AddCanvasOptions;
let parent = Canvas::new_blank("Parent", 800, 600);
let child = Canvas::new_from_path("Child", "assets/photo.jpg", None);
parent.add_canvas(child, Some(AddCanvasOptions::new().with_rotation(10.0)));
```

### anchor_to_canvas
Stores an anchor to position this canvas inside its parent during composition.
```rust
use abra::Anchor;
canvas.anchor_to_canvas(Anchor::BottomRight);
```

### get_layer_by_index / get_layer_by_name
Retrieves layers by index or name.
```rust
let first = canvas.get_layer_by_index(0);
let logo = canvas.get_layer_by_name("Logo");
```

### layers / layer_count
Lists all layers or returns their count.
```rust
let count = canvas.layer_count();
let all = canvas.layers();
```

### transform
Returns a `CanvasTransform` proxy for bulk transforms.
```rust
let mut t = canvas.transform();
t.resize(1024, 768, None).crop(10, 10, 1000, 700);
```
