---
title: Positioning
outline: deep
order: 4
---

# Positioning

Canvas positioning uses two concepts:

- `Anchor` chooses a point in the parent canvas.
- `Origin` chooses the point inside the child layer or canvas that aligns to that anchor.

Together they let you place content predictably without calculating its top-left coordinates manually.

## Anchors

`Anchor` provides nine positions in the parent:

- `TopLeft`, `TopCenter`, `TopRight`
- `CenterLeft`, `Center`, `CenterRight`
- `BottomLeft`, `BottomCenter`, `BottomRight`

```rust
use abra::canvas::Anchor;

let logo = canvas.get_layer_by_name("Logo").unwrap();
logo.anchor_to_canvas(Anchor::TopRight);
```

For example, `Anchor::Center` places the child so its anchor point is centered in the parent. `Anchor::BottomRight` aligns it to the parent's lower-right corner.

Anchors can be set when adding a layer:

```rust
use abra::canvas::{Anchor, NewLayerOptions};

let options = NewLayerOptions::new()
  .with_anchor(Anchor::BottomRight);
canvas.add_layer_from_path("Logo", "assets/logo.png", Some(options));
```

## Origins

`Origin` selects which point inside the layer is aligned to the anchor. The default is `Origin::Center`:

```rust
use abra::canvas::{Anchor, Origin};

let layer = canvas.get_layer_by_name("Badge").unwrap();
layer.anchor_to_canvas(Anchor::Center);
layer.set_origin(Origin::TopLeft);
```

Available named origins mirror the anchor grid. `Origin::Custom(x, y)` uses normalized coordinates inside the layer, where `0.0` is the left or top edge and `1.0` is the right or bottom edge:

```rust
layer.set_origin(Origin::Custom(0.25, 0.75));
```

`Origin::Custom(0.5, 0.5)` is equivalent to `Origin::Center`.

## Absolute position

Use `set_global_position` when the layer or canvas should use explicit coordinates:

```rust
layer.set_global_position(120, 40);
let (x, y) = layer.position();

canvas.set_position(80, 20);
let canvas_position = canvas.position();
```

Absolute positions are relative to the containing canvas. Setting an explicit position is useful for pixel-precise layouts and offsets after anchoring.

## Relative layer position

Position a layer relative to another layer's current position:

```rust
let photo = canvas.get_layer_by_name("Photo").unwrap();
let caption = canvas.get_layer_by_name("Caption").unwrap();
caption.set_relative_position(20, -20, &photo);
```

The offset is added to the referenced layer's position. It does not resize or anchor either layer.

## Child canvases

A child canvas can be anchored inside its parent just like a layer:

```rust
use abra::canvas::Anchor;

child.anchor_to_canvas(Anchor::Center);
parent.add_canvas(child, None);
```

Use `canvas.set_origin` on a child canvas when a different internal reference point should align to the parent anchor.

## Positioning summary

| API                                  | Purpose                                          |
| ------------------------------------ | ------------------------------------------------ |
| `anchor_to_canvas(Anchor)`           | Store a parent-relative anchor.                  |
| `set_origin(Origin)`                 | Choose the internal point aligned to the anchor. |
| `set_global_position(x, y)`          | Set an explicit parent-relative position.        |
| `position()`                         | Read the current position.                       |
| `set_relative_position(x, y, layer)` | Offset a layer from another layer.               |
