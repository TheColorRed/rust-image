---
title: Image transforms
order: 1
outline: deep
---

# Image transforms

The `Resize`, `Crop`, and `Rotate` traits provide image transformations as in-place methods.

## Resize

Resize to exact dimensions, or preserve the aspect ratio by specifying one dimension:

```rust
use abra::prelude::*;
use abra::abra_core::{FlipAxis, ResizeTarget, Size};

let mut image = Image::read("assets/photo.jpg")?;
image.resize(ResizeTarget::Exact(Size::new(1200, 800)), Some(TransformAlgorithm::Lanczos));
image.resize(ResizeTarget::FitWidth(640), None);
image.resize(ResizeTarget::FitHeight(480), None);
```

`ResizeTarget::Exact` can stretch the image when the target aspect ratio differs. `FitWidth` and `FitHeight` calculate the other dimension from the original ratio.

Resize relative to the current dimensions or by a percentage:

```rust
image.resize(ResizeTarget::Scale(0.5), Some(TransformAlgorithm::Bicubic));
image.resize(ResizeTarget::RelativeWidth(120), None);
image.resize(ResizeTarget::RelativeHeight(-40), None);
```

A percentage of `0.5` means 50% of the current image size. Relative values are pixel changes.

## Resize algorithms

Choose an algorithm based on the quality and speed needed:

| Algorithm         | Behavior                                                  |
| ----------------- | --------------------------------------------------------- |
| `NearestNeighbor` | Fastest; preserves hard pixel edges but can look blocky.  |
| `Bilinear`        | Smooth and inexpensive; useful for general resizing.      |
| `Bicubic`         | Higher quality using a broader cubic neighborhood.        |
| `Lanczos`         | Highest-quality standard interpolation; more expensive.   |
| `EdgeDirectEDI`   | Edge-directed resizing with lower cost than NEDI.         |
| `EdgeDirectNEDI`  | Edge-directed resizing with stronger detail preservation. |
| `Auto`            | Select an algorithm based on the image and target size.   |

Pass `None` to let the implementation choose automatically.

## Crop

Crop from an origin and replace the image with the selected rectangle:

```rust
use abra::transform::prelude::Crop;

image.crop(100, 60, 800, 500);
```

The arguments are `x`, `y`, `width`, and `height` in source-image pixels. Use `cropped` when a new image is preferred:

```rust
use abra::transform::prelude::cropped;

let result = cropped(&image, 100, 60, 800, 500);
```

Choose coordinates and dimensions within the source image bounds.

## Rotate

Rotate clockwise by a positive angle and counter-clockwise by a negative angle:

```rust
use abra::transform::prelude::Rotate;

image.rotate(12.0, Some(TransformAlgorithm::Bilinear));
```

Rotation expands the image to contain the rotated result. Transparent pixels are used outside the source bounds, which avoids opaque borders during interpolation.

## Flip

Flip in place horizontally or vertically:

```rust
flip(FlipAxis::Horizontal).apply(&mut image);
flip(FlipAxis::Vertical).apply(&mut image);
```

Calling the same flip twice restores the original orientation.

## Zoom

Zoom toward an anchor point while keeping the original dimensions. The anchor stays in place, like zooming toward a cursor:

```rust
zoom((320.0, 180.0), 2.0).apply(&mut image);
zoom((320.0, 180.0), 2.0).with_algorithm(TransformAlgorithm::Lanczos).apply(&mut image);
```

A factor of `1.0` or less leaves the image unchanged. The algorithm is chosen automatically unless set with `with_algorithm`.

## API summary

| API                                        | Purpose                                      |
| ------------------------------------------ | -------------------------------------------- |
| `resize(ResizeTarget::Exact(...), algorithm)` | Resize to exact dimensions.                |
| `resize(ResizeTarget::Scale(...), algorithm)` | Scale by a factor.                          |
| `resize(ResizeTarget::FitWidth(...), algorithm)` | Resize width while preserving aspect ratio. |
| `resize(ResizeTarget::FitHeight(...), algorithm)` | Resize height while preserving aspect ratio. |
| `resize(ResizeTarget::RelativeWidth(...), algorithm)` | Change width by a pixel delta.     |
| `resize(ResizeTarget::RelativeHeight(...), algorithm)` | Change height by a pixel delta.   |
| `crop(x, y, width, height)`                | Crop in place.                               |
| `cropped(image, ...)`                      | Return a cropped image.                      |
| `rotate(degrees, algorithm)`               | Rotate and expand to fit.                    |
| `flip(FlipAxis).apply(image)`              | Mirror the image.                            |
| `zoom(anchor, factor).apply(image)`        | Zoom toward a point, keeping the size.       |
