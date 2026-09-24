---
title: Perspective
order: 3
outline: deep
---

# Perspective

`perspective` corrects the perspective of an image. Give it an area that should be a rectangle but was photographed at an angle, such as a building, a sign, or a page, and the image is warped so the area's four corners become the corners of a rectangle.

```rust
use abra::prelude::*;
use abra::tools::prelude::*;

let mut image = Image::read("assets/page.jpg")?;

let area = Area::from_points(&[[150.0, 500.0], [800.0, 500.0], [650.0, 1100.0], [0.0, 1150.0]]);

perspective(area).apply(&mut image);
```

The rectangle is at least as wide and tall as the longer of each pair of opposite sides, and shaped so that nothing changes shape at the corner where the two longest sides meet, the part of the area nearest the camera. Sized by the sides alone, the far end would come out squashed, like the top of a building shot from its foot, which is foreshortened in the photo.

## The area

The area must be a polygon with exactly four corners that form a convex shape. The corners can be given in any order. Anything else, such as a circle, a shape with curves, or a shape with a dent, leaves the image unchanged.

The builder methods on `Area` borrow it, so build an owned area in its own statement:

```rust
let mut area = Area::new();
area
  .move_to(PointF::new(0, 250))
  .line_to(PointF::new(1200, 75))
  .line_to(PointF::new(1200, 900))
  .line_to(PointF::new(0, 900));

perspective(area).apply(&mut image);
```

## Options

| Method                             | Default   | Behavior                                                                      |
| ---------------------------------- | --------- | ----------------------------------------------------------------------------- |
| `with_crop(bool)`                  | `true`    | Keep only the flattened area. `false` stretches it to its bounding box in place. |
| `with_algorithm(TransformAlgorithm)` | `Lanczos` | Interpolation used to resample. The edge-directed algorithms use Lanczos here. |

## Cropping

With `with_crop(true)` the result is exactly the flattened area.

With `with_crop(false)` the whole image is warped and stays the same size as the original, and the area is stretched out in place to fill its bounding box. Each corner only moves outwards, and no further than the corners next to it already reach:

| Corner       | Moves                                                                      |
| ------------ | -------------------------------------------------------------------------- |
| Top-left     | Up, no higher than the top-right. Left, no further than the bottom-left.   |
| Top-right    | Up, no higher than the top-left. Right, no further than the bottom-right.  |
| Bottom-left  | Down, no lower than the bottom-right. Left, no further than the top-left.  |
| Bottom-right | Down, no lower than the bottom-left. Right, no further than the top-right. |

For a building photographed from below, the bottom and sides stay where they are and the top is pulled outwards until the walls are upright.

Stretching the area out also stretches what lies beyond it, and beyond its longer sides that pulls in parts of the view from past the edges of the photo. The result then zooms in, keeping its size and aspect ratio, just far enough that every pixel comes from the photo. Nothing is smeared out from the edges and no transparent gaps open up.

```rust
perspective(area).with_crop(false).with_algorithm(TransformAlgorithm::Bicubic).apply(&mut image);
```
