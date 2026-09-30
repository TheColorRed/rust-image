---
title: Layer
outline: deep
order: 2
---

# Layer

A `Layer` is an image inside a [`Canvas`](./canvas). Layers provide stacking, visibility, opacity, blending, positioning, transforms, and effects.

Layers are normally created with `Canvas::add_layer_from_path` or `Canvas::add_layer_from_image` rather than constructed directly.

## Creating and accessing layers

```rust
use abra::canvas::prelude::*;

let canvas = Canvas::new_blank("Composition", 800, 600);
canvas.add_layer_from_path("Photo", "assets/photo.jpg", None);
canvas.add_layer_from_path("Mark", "assets/mark.png", None);

let photo = canvas.get_layer_by_name("Photo").unwrap();
let mark = canvas.get_layer_by_name("Mark").unwrap();
```

Read the layer name, stable ID, dimensions, and current stack index:

```rust
let name = photo.name();
let id = photo.id();
let (width, height) = photo.dimensions();
let index = photo.current_index();
```

## Visibility, opacity, and blending

```rust
photo.set_visible(true);
photo.set_opacity(0.85);

let visible = photo.is_visible();
let opacity = photo.opacity();
```

Set a blend mode with `set_blend_mode`. See [Blend modes](./blend-modes) for every mode:

```rust
use abra::abra_core::BlendMode;

photo.set_blend_mode(BlendMode::Multiply);
let current_blend = photo.blend_mode();
```

Opacity is expressed from `0.0` (invisible) to `1.0` (fully opaque).

## Positioning

Set an absolute position, position relative to another layer, or anchor a layer to the canvas:

```rust
use abra::canvas::{Anchor, Origin};

photo.set_global_position(120, 40);
mark.set_relative_position(20, -10, &photo);
mark.anchor_to_canvas(Anchor::TopRight);
mark.set_origin(Origin::TopRight);
```

See [Positioning](./positioning) for anchor and origin behavior.

## Layer order

Layers are composited in stack order. Move a layer one step or place it at an end or exact index:

```rust
mark.move_to(LayerMove::Up);
mark.move_to(LayerMove::Down);
mark.move_to(LayerMove::Top);
mark.move_to(LayerMove::Bottom);
mark.set_index(1);
```

Operations at an existing boundary have no effect. Use `current_index` to inspect the resulting order.

## Image access

Get a flattened copy of the layer image with `as_image`, or work with the image through a closure:

```rust
let image = photo.as_image();
image.write("out/photo-layer.png", None)?;

photo.with_image_mut(|image| {
  image.write("out/photo-layer-copy.png", None)?;
});
```

`with_image_mut` acquires the layer lock for the duration of the closure and marks the parent canvas for recomposition when layer methods modify state.

## Transforms

Use `transform()` for resize, crop, rotation, and flips:

```rust
use abra::abra_core::{FlipAxis, ResizeTarget};

photo.transform()
  .resize(ResizeTarget::FitWidth(640), None)
  .crop(0, 0, 640, 360)
  .rotate(2.0, None)
  .flip(FlipAxis::Horizontal);
```

See [Transforms](./transforms) for method details.

## Duplicating layers

`duplicate` creates a copy in the same canvas:

```rust
let copy = photo.duplicate();
copy.set_name("Photo Copy");
copy.set_opacity(0.5);
```

## Layer effects

Queue effects with the builder returned by `effects`, or set a complete `LayerEffects` value with `set_effects`:

```rust
use abra::canvas::effects::DropShadow;

photo.effects()
  .with_drop_shadow(DropShadow::new().with_distance(8.0).with_size(6.0));
```

The builder commits when it is dropped. See [Effects](./effects).

## API summary

| API                                                                  | Purpose                                  |
| -------------------------------------------------------------------- | ---------------------------------------- |
| `name`, `set_name`, `id`                                             | Identify a layer.                        |
| `dimensions`                                                         | Read the layer size.                     |
| `set_visible`, `is_visible`                                          | Control visibility.                      |
| `set_opacity`, `opacity`                                             | Control transparency.                    |
| `set_blend_mode`, `blend_mode`                                       | Control compositing.                     |
| `set_global_position`, `position`                                    | Set or read absolute position.           |
| `set_relative_position`                                              | Place a layer relative to another layer. |
| `anchor_to_canvas`, `set_origin`                                     | Configure anchor positioning.            |
| `move_to(LayerMove)`, `set_index`                                    | Reorder the layer stack.                 |
| `duplicate`                                                          | Copy a layer.                            |
| `transform`                                                          | Transform the layer image.               |
| `effects`, `set_effects`                                             | Configure render-time effects.           |
