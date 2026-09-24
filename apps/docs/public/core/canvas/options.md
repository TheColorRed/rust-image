---
title: Layer options
outline: deep
order: 3
---

# Layer options

`NewLayerOptions` controls how an image is configured when it is added to a canvas. Pass it to `add_layer_from_path`, `add_layer_from_image`, or `Canvas::new_from_path`.

```rust
use abra::canvas::{Anchor, LayerSize, NewLayerOptions};

let options = NewLayerOptions::new()
  .with_size(LayerSize::Cover(None))
  .with_anchor(Anchor::Center)
  .with_opacity(0.85);
```

## Defaults

`NewLayerOptions::new()` defaults to:

- `Anchor::Center`
- opacity `1.0`
- the normal blend mode
- `LayerSize::Maintain`

If no options are supplied, layers keep their original image size and use these defaults.

## Layer size

`LayerSize` controls how the source image is resized when it is added:

| Variant                              | Behavior                                                                          |
| ------------------------------------ | --------------------------------------------------------------------------------- |
| `Maintain`                           | Keep the original image dimensions.                                               |
| `Contain(algorithm)`                 | Scale to fit inside the canvas without cropping or stretching.                    |
| `Cover(algorithm)`                   | Scale to cover the canvas while preserving aspect ratio; overflow may be cropped. |
| `Specific(width, height, algorithm)` | Resize to exact pixel dimensions.                                                 |
| `Percentage(amount, algorithm)`      | Resize relative to the original size. `0.5` means 50%.                            |

The algorithm is optional. Pass `None` to use the default resize algorithm:

```rust
use abra::canvas::{LayerSize, NewLayerOptions};

let fit = NewLayerOptions::new().with_size(LayerSize::Contain(None));
let cover = NewLayerOptions::new().with_size(LayerSize::Cover(None));
let exact = NewLayerOptions::new().with_size(LayerSize::Specific(800, 600, None));
let half = NewLayerOptions::new().with_size(LayerSize::Percentage(0.5, None));
```

## Anchoring

Use `with_anchor` to place the layer relative to the canvas:

```rust
use abra::canvas::{Anchor, NewLayerOptions};

let options = NewLayerOptions::new()
  .with_anchor(Anchor::BottomRight);
```

See [Positioning](./positioning) for all anchor and origin variants.

## Opacity

Use `with_opacity` to set transparency. Values are clamped to `0.0..=1.0`:

```rust
let options = NewLayerOptions::new().with_opacity(0.6);
```

## Blend mode

Pass a blend function with `with_blend_mode`:

```rust
use abra::abra_core::blend;
use abra::canvas::NewLayerOptions;

let options = NewLayerOptions::new()
  .with_blend_mode(blend::multiply);
```

## Adding a configured layer

```rust
use abra::canvas::{Anchor, Canvas, LayerSize, NewLayerOptions};

let canvas = Canvas::new_blank("Poster", 1200, 800);
let options = NewLayerOptions::new()
  .with_size(LayerSize::Cover(None))
  .with_anchor(Anchor::Center)
  .with_opacity(0.9);

canvas.add_layer_from_path("Photo", "assets/photo.jpg", Some(options));
canvas.save("out/poster.png", None);
```

## API summary

| API                                       | Purpose                                |
| ----------------------------------------- | -------------------------------------- |
| `NewLayerOptions::new()`                  | Create options with standard defaults. |
| `with_size(LayerSize)`                    | Set the layer resizing strategy.       |
| `with_anchor(Anchor)`                     | Set the layer's canvas anchor.         |
| `with_opacity(f32)`                       | Set opacity, clamped to `0.0..=1.0`.   |
| `with_blend_mode(fn(RGBA, RGBA) -> RGBA)` | Set the compositing function.          |
