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
| `with_orientation(Orientation)`    | `Nearest`    | The direction the line is turned to. See [Orientation](#orientation).          |
| `with_fit(TransformFit)`              | `Crop`       | How the canvas is sized after rotating. See [Fit](#fit).                      |
| `with_algorithm(TransformAlgorithm)` | `Lanczos`    | Interpolation used to rotate. Lanczos is the highest quality and the slowest. |

## Orientation

| Value                         | Result                                                                                   |
| ----------------------------- | ---------------------------------------------------------------------------------------- |
| `Orientation::Nearest`        | Whichever of horizontal or vertical is closer, so the image turns as little as possible. |
| `Orientation::Horizontal`     | The line becomes horizontal.                                                             |
| `Orientation::Vertical`       | The line becomes vertical.                                                               |
| `Orientation::Angle(degrees)` | The line is turned to any other angle, measured clockwise from horizontal.               |

```rust
straighten(PointF::new(350, 300), PointF::new(450, 250))
  .with_orientation(Orientation::Horizontal)
  .with_algorithm(TransformAlgorithm::Bilinear)
  .apply(&mut image);
```

## Fit

Straightening uses the same fit modes as [`rotate`](../core/transform/image-transforms.md#rotate).

| Value               | Result                                                                                                           |
| ------------------- | ---------------------------------------------------------------------------------------------------------------- |
| `TransformFit::Crop`   | The largest area with no empty corners, which is smaller than the original. Default.                             |
| `TransformFit::Fill`   | The largest area with the original aspect ratio, scaled back up to the original size. The image is never stretched. |
| `TransformFit::Expand` | The whole rotated image, on a larger canvas with transparent corners.                                            |

```rust
straighten(start, end).with_fit(TransformFit::Fill).apply(&mut image);
```

## Notes

- Both points are in image pixels, with the origin at the top-left corner.
- Two identical points have no direction, so the image is left unchanged.
