---
title: Transforms
outline: deep
order: 5
---

# Transforms

Canvas and layer transforms provide resize, crop, rotation, and flip operations. The transform APIs mutate the underlying image and mark the composition for recomposition.

## Layer transforms

Call `layer.transform()` to get a `LayerTransform` proxy. Its methods come from the `Transform` trait, the same trait `Image` implements:

```rust
use abra::abra_core::{FlipAxis, ResizeTarget, Size, Transform};

let photo = canvas.get_layer_by_name("Photo").unwrap();

let mut transform = photo.transform();
transform.resize(ResizeTarget::FitWidth(640), None);
transform.crop(0, 0, 640, 360);
transform.rotate(2.0, None);
```

### Resize

Resize to exact dimensions, preserve a percentage of the original size, or change one dimension:

```rust
let mut transform = photo.transform();
transform.resize(ResizeTarget::Exact(Size::new(800, 600)), None);
transform.resize(ResizeTarget::Scale(0.5), None);
transform.resize(ResizeTarget::FitWidth(640), None);
transform.resize(ResizeTarget::FitHeight(360), None);
```

Relative resize methods add or subtract pixels from the current dimension:

```rust
transform.resize(ResizeTarget::RelativeWidth(80), None);
transform.resize(ResizeTarget::RelativeHeight(-20), None);
```

The optional algorithm accepts a `TransformAlgorithm`. Pass `None` to use the default.

### Crop

Crop the layer to a rectangle:

```rust
photo.transform().crop(40, 20, 640, 360);
```

The arguments are `x`, `y`, `width`, and `height` in image coordinates.

### Rotate and flip

Rotate by degrees and flip around the horizontal or vertical axis:

```rust
let mut transform = photo.transform();
transform.rotate(5.0, None);
transform.flip(FlipAxis::Horizontal);
transform.flip(FlipAxis::Vertical);
```

## Canvas transforms

Use `canvas.transform()` for transforms that apply to the composed canvas:

```rust
let mut transform = canvas.transform();
transform.resize(CanvasResizeTarget::Exact(Size::new(1200, 800)), None);
transform.crop(0, 0, 1200, 720);
```

Canvas transforms are useful for changing the output dimensions after the layer composition has been assembled.

## Layer size options versus transforms

Use [`LayerSize`](./options) when configuring an image as it enters a canvas. Use transforms when modifying a layer or canvas after it exists:

| Need                                   | API                             |
| -------------------------------------- | ------------------------------- |
| Fit a new layer inside the canvas      | `LayerSize::Contain(...)`       |
| Cover a new canvas without empty space | `LayerSize::Cover(...)`         |
| Set a new layer's exact size           | `LayerSize::Specific(...)`      |
| Resize an existing layer               | `layer.transform().resize(...)` |
| Crop an existing layer                 | `layer.transform().crop(...)`   |
| Resize or crop the composed canvas     | `canvas.transform()`            |

## API summary

| API                                        | Purpose                              |
| ------------------------------------------ | ------------------------------------ |
| `resize(ResizeTarget::Exact(...), algorithm)` | Resize to exact dimensions.       |
| `resize(ResizeTarget::Scale(...), algorithm)` | Resize relative to the current size. |
| `resize(ResizeTarget::FitWidth(...), algorithm)` | Set image width while preserving aspect ratio. |
| `resize(ResizeTarget::FitHeight(...), algorithm)` | Set image height while preserving aspect ratio. |
| `resize(ResizeTarget::RelativeWidth(...), algorithm)` | Adjust image width by pixels. |
| `resize(ResizeTarget::RelativeHeight(...), algorithm)` | Adjust image height by pixels. |
| `crop(x, y, width, height)`                | Crop to a rectangle.                 |
| `rotate(degrees, algorithm)`               | Rotate the image.                    |
| `flip(FlipAxis::Horizontal)`               | Mirror horizontally.                 |
| `flip(FlipAxis::Vertical)`                 | Mirror vertically.                   |
