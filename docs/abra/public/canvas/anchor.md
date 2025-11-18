# Anchor

Predefined positions relative to a parent area.

Variants: `TopLeft`, `TopCenter`, `TopRight`, `CenterLeft`, `Center`, `CenterRight`, `BottomLeft`, `BottomCenter`, `BottomRight`.

- Used by `Layer::anchor_to_canvas` and `Canvas::anchor_to_canvas`.
- Positions content based on parent and child dimensions.

Example: center a layer on the canvas.
```rust
use abra::Anchor;
let layer = canvas.get_layer_by_index(0).unwrap();
layer.anchor_to_canvas(Anchor::Center);
```
