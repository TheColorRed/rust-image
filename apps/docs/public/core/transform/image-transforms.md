---
title: Image transforms
order: 1
outline: deep
---

# Image transforms

`transform()` crops, resizes, rotates and flips an image in one go. Describe what to do with the `with_*` methods and run it with `apply`; the library picks the order the pieces run in, so the picture is resampled as little as possible. `warp` and `zoom` are separate builders. The `Transform` trait (`resize`, `crop`, `rotate`, `flip`) is what layers and canvases offer for their contents; an `Image` is changed with `transform()`.

```rust
use abra::prelude::*;
use abra::abra_core::{FlipAxis, ResizeTarget, Size};

let mut image = Image::read("assets/photo.jpg")?;
transform()
  .with_crop(Rect::new((100, 60), (800, 500)))
  .with_scale(2.0)
  .with_rotation(12.0)
  .with_algorithm(TransformAlgorithm::Bicubic)
  .apply(&mut image);
```

`applied(&image)` returns a changed copy and leaves the original as it was.

## The order

You do not choose the order, because the right one depends on the sizes:

- A **crop** only copies pixels, so it is done first and the rest works on fewer pixels.
- A **shrink** is made before a rotation, by the resize, which looks at every pixel it replaces. The rotation then works on the smaller picture.
- An **enlargement** is made together with the rotation, each pixel read once from the original, so a picture that is zoomed and turned is as sharp as one that is only zoomed.
- A **flip** is an exact mirror, done on the smaller picture.

## Resize

`with_size` takes a `ResizeTarget`, and `with_scale(0.5)` is a shortcut for `ResizeTarget::Scale`:

```rust
transform().with_size(ResizeTarget::Exact(Size::new(1200, 800))).with_algorithm(TransformAlgorithm::Lanczos).apply(&mut image);
transform().with_size(ResizeTarget::FitWidth(640)).apply(&mut image);
transform().with_size(ResizeTarget::FitHeight(480)).apply(&mut image);
```

`ResizeTarget::Exact` can stretch the image when the target aspect ratio differs. `FitWidth` and `FitHeight` calculate the other dimension from the original ratio.

Resize relative to the current dimensions or by a factor:

```rust
transform().with_scale(0.5).apply(&mut image);
transform().with_size(ResizeTarget::RelativeWidth(120)).apply(&mut image);
transform().with_size(ResizeTarget::RelativeHeight(-40)).apply(&mut image);
```

A factor of `0.5` means 50% of the current image size. Relative values are pixel changes.

## Resize algorithms

Choose an algorithm based on the quality and speed needed:

| Algorithm         | Behavior                                                  |
| ----------------- | --------------------------------------------------------- |
| `NearestNeighbor` | Fastest; preserves hard pixel edges but can look blocky.  |
| `Bilinear`        | Smooth and inexpensive; useful for general resizing.      |
| `Bicubic`         | Higher quality using a broader cubic neighborhood.        |
| `Lanczos`         | Highest-quality standard interpolation; more expensive.   |
| `EdgeDirectEDI`   | Edge-directed interpolation with lower cost than NEDI.    |
| `EdgeDirectNEDI`  | Edge-directed interpolation with stronger detail preservation. |
| `Auto`            | Select an algorithm based on the image and target size.   |

Pass `None` to `with_algorithm`, or leave it out, to let the implementation choose automatically. The same algorithms apply to rotation and warping, since all geometric transforms sample the source through the same `remap` loop.

## Crop

`with_crop` keeps an area of the image, in source pixels. A `Size` keeps that size from the top-left corner:

```rust
transform().with_crop(Rect::new((100, 60), (800, 500))).apply(&mut image);
```

The area is clipped to the image; an area entirely outside the image leaves it unchanged. A size, scale or rotation in the same transform works on the cropped picture.

## Rotate

`with_rotation` turns clockwise by a positive angle and counter-clockwise by a negative angle:

```rust
transform().with_rotation(12.0).with_algorithm(TransformAlgorithm::Bilinear).apply(&mut image);
```

Rotation expands the image to contain the rotated result. Transparent pixels are used outside the source bounds, which avoids opaque borders during interpolation.

`with_fit` chooses how the canvas is sized:

```rust
transform().with_rotation(12.0).with_fit(TransformFit::Crop).apply(&mut image);
transform().with_rotation(12.0).with_fit(TransformFit::Fill).with_algorithm(TransformAlgorithm::Lanczos).apply(&mut image);
```

| Value               | Result                                                                                                                             |
| ------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| `TransformFit::Expand` | Grow the canvas to fit the whole rotated image, with transparent corners. Default.                                                 |
| `TransformFit::Crop`   | Keep the largest rectangle with no transparent corners. The result is smaller than the original.                                   |
| `TransformFit::Fill`   | Keep the largest rectangle with the original aspect ratio and scale it back up to the original size, in a single resampling pass. |

To preview the area `Crop` or `Fill` will keep, for example to draw a crop overlay, use `Size::inscribed_after_rotation`. It returns the kept size, centered on the rotated image, and can also find the largest rectangle of any other aspect ratio:

```rust
let size = Size::new(4000, 3000);
let crop = size.inscribed_after_rotation(12.0, None);                // TransformFit::Crop
let fill = size.inscribed_after_rotation(12.0, size.aspect_ratio()); // TransformFit::Fill
let wide = size.inscribed_after_rotation(12.0, 16.0 / 9.0);          // a 16:9 crop
```

## Warp

Move four points of the image onto four other points with a perspective transform. Straight lines stay straight, the way a flat surface changes when seen from another angle:

```rust
let from = Quad::new((120, 80), (900, 140), (880, 700), (100, 760));
let to = Quad::rect((0, 0), (800, 600));
warp(from, to).with_fit(TransformFit::Crop).apply(&mut image);
```

Both arguments accept anything that converts into a `Quad`, including a `Rect` such as `quad.bounds()`.

`with_fit` works as it does for rotation:

| Value                  | Result                                                                                                       |
| ---------------------- | ------------------------------------------------------------------------------------------------------------ |
| `TransformFit::Expand` | The whole warped image, with transparent gaps. The canvas grows at most half the image's size past each edge. Default. |
| `TransformFit::Crop`   | Only the upright box around the `to` quad.                                                                   |
| `TransformFit::Fill`   | The original frame, zoomed in just enough that every pixel comes from the source.                            |

`Homography::from_quads(&from, &to)` gives the underlying transform, with `map` to move single points.

## Flip

`with_flip` mirrors horizontally or vertically, before any rotation:

```rust
transform().with_flip(FlipAxis::Horizontal).apply(&mut image);
transform().with_flip(FlipAxis::Vertical).apply(&mut image);
```

Flipping the same axis twice in one transform restores the original orientation.

## Zoom

Zoom toward an anchor point while keeping the original dimensions. The anchor stays in place, like zooming toward a cursor:

```rust
zoom((320.0, 180.0), 2.0).apply(&mut image);
zoom((320.0, 180.0), 2.0).with_algorithm(TransformAlgorithm::Lanczos).apply(&mut image);
```

A factor of `1.0` or less leaves the image unchanged. The algorithm is chosen automatically unless set with `with_algorithm`.

## API summary

| API                                        | Purpose                                                          |
| ------------------------------------------ | ---------------------------------------------------------------- |
| `transform()`                              | Start a transform; run it with `apply`, or `applied` for a copy. |
| `.with_crop(area)`                         | Keep an area of the image.                                       |
| `.with_size(ResizeTarget)`                 | Resize to a target size.                                         |
| `.with_scale(factor)`                      | Scale both dimensions by a factor.                               |
| `.with_rotation(degrees)`                  | Rotate clockwise about the middle.                               |
| `.with_flip(FlipAxis)`                     | Mirror the image.                                                |
| `.with_fit(TransformFit)`                  | Size the canvas after a rotation.                                |
| `.with_algorithm(algorithm)`               | Choose the interpolation; automatic when not set.                |
| `.with_hardware(Hardware)`                 | Run on the CPU, the GPU, or as the settings say.                 |
| `warp(from, to).apply(image)`              | Move four points onto four others.                               |
| `zoom(anchor, factor).apply(image)`        | Zoom toward a point, keeping the size.                           |
