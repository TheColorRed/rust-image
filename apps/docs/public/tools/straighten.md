---
title: Straighten
order: 2
outline: deep
---

# Straighten

`straighten` rotates an image until a line you pick becomes horizontal or vertical. Pick two points along something that should be level or plumb, such as a horizon, a shelf, or a door frame.

```rust
use abra::prelude::*;
use abra::tools::prelude::*;

let mut image = Image::read("assets/photo.jpg")?;

straighten(PointF::new(100, 950), PointF::new(600, 1050)).apply(&mut image);
```

The image rotates around its center. Rotating leaves transparent corners, so by default they are cropped away.

## Options

| Method                             | Default      | Behavior                                                                      |
| ---------------------------------- | ------------ | ----------------------------------------------------------------------------- |
| `with_axis(Axis)`                  | `Axis::Auto` | Whether the line becomes horizontal, vertical, or whichever is closer.        |
| `with_crop(bool)`                  | `true`       | Crop away the empty corners the rotation leaves.                              |
| `with_resize(bool)`                | `false`      | Scale the cropped result back up to the original size.                        |
| `with_algorithm(TransformAlgorithm)` | `Lanczos`    | Interpolation used to rotate. Lanczos is the highest quality and the slowest. |

## Axis

| Value              | Result                                                                                   |
| ------------------ | ---------------------------------------------------------------------------------------- |
| `Axis::Horizontal` | The line becomes horizontal.                                                             |
| `Axis::Vertical`   | The line becomes vertical.                                                               |
| `Axis::Auto`       | Whichever of horizontal or vertical is closer, so the image turns as little as possible. |

```rust
straighten(PointF::new(350, 300), PointF::new(450, 250))
  .with_axis(Axis::Horizontal)
  .with_algorithm(TransformAlgorithm::Bilinear)
  .apply(&mut image);
```

## Cropping and resizing

With `with_crop(true)` the result is the largest area with no empty corners, which is smaller than the original. Add `with_resize(true)` to scale that area back up to the original size while keeping the original aspect ratio, so the image is never stretched.

With `with_crop(false)` the whole rotated image is kept on a larger canvas with transparent corners. `with_resize` only applies when cropping, because scaling a canvas of a different shape to the original size would stretch it.

```rust
straighten(start, end).with_crop(true).with_resize(true).apply(&mut image);
```

## Notes

- Both points are in image pixels, with the origin at the top-left corner.
- Two identical points have no direction, so the image is left unchanged.
