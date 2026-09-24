---
title: Remover
order: 1
outline: deep
---

# Remover

`remover` heals blemishes, spots, and scratches, like a spot-healing brush. Choose a shape and where to place it, and the shape is filled in from the surrounding image. The texture comes from a nearby patch, and the color and lighting come from the pixels around the edge, so the repair blends in.

```rust
use abra::prelude::*;
use abra::tools::prelude::*;

let mut image = Image::read("assets/portrait.jpg")?;

remover(RemoverShape::circle(12))
  .with_position((338, 1694))
  .apply(&mut image);
```

A remover has a **shape**, which has a size but no location, and one or more **positions**, which say where the shape is placed. With no positions, applying the remover changes nothing.

## Shapes

| Constructor                              | Shape                                                                    |
| ---------------------------------------- | ------------------------------------------------------------------------ |
| `RemoverShape::circle(radius)`           | A circle. Dragged along a path, it becomes a brush stroke.               |
| `RemoverShape::rectangle(width, height)` | A rectangle, centered on its position.                                   |
| `RemoverShape::area(area)`               | Any `Area`, placed so the middle of its bounding box is on the position. |

Sizes are in pixels and accept any number type.

```rust
remover(RemoverShape::rectangle(28, 28)).with_position((338, 1694)).apply(&mut image);

let mut blob = Area::new();
blob
  .move_to(PointF::new(0, 0))
  .line_to(PointF::new(30, 5))
  .line_to(PointF::new(20, 30));
remover(RemoverShape::area(blob)).with_position((613, 1767)).apply(&mut image);
```

## Positions

Positions say where the shape is placed. Add as many as you need, and the remover heals at each one in the order they were added, so a later position sees the result of the earlier ones.

| Method             | Position                                                  |
| ------------------ | --------------------------------------------------------- |
| `with_position(p)` | A point, a line between two points, or a `ShapePosition`. |
| `with_path(path)`  | A path through points, or a `Path`, healed as one stroke. |

### Points and lines

`with_position` accepts a point as `(x, y)` or a `PointF`, and a line as `((x1, y1), (x2, y2))` or a pair of `PointF`s. Coordinates can be any number type.

```rust
remover(RemoverShape::circle(10))
  .with_position((338, 1694))                // a single spot
  .with_position(((894, 156), (933, 123)))   // a circle dragged from one point to another
  .apply(&mut image);
```

### Paths

`with_path` follows a series of points joined by straight lines, or a `Path` with curves. The whole path is healed as one stroke, so it is faster than healing a line at a time and has no seams where the segments meet.

```rust
// Points, in an array, a Vec, or a slice.
remover(RemoverShape::circle(5))
  .with_path([(894, 156), (909, 141), (918, 132), (933, 123)])
  .apply(&mut image);

// An existing path, curves included.
let path = Path::line((100, 100), (200, 200));
remover(RemoverShape::circle(8)).with_path(&path).apply(&mut image);
```

One point is a single dot, and no points adds nothing. Curves are flattened to straight lines that stay within a quarter of a pixel of the curve.

Circles follow a path. Rectangles and areas cannot be dragged along one, so they are centered on the middle of the path's bounding box.

## Options

| Method                     | Default     | Behavior                                                  |
| -------------------------- | ----------- | --------------------------------------------------------- |
| `with_distance(Units)`     | `Pixels(3)` | How far around the shape to sample the surrounding image. |
| `with_hardness(0.0..=1.0)` | `0.8`       | How sharply the repair reaches the edge of the shape.     |

### Distance

The distance is the width of the ring of pixels just outside the shape. The remover matches the repair to that ring, and uses it to choose which nearby patch to borrow texture from. A wider ring gives a better match on textured surfaces, but includes more of the surroundings.

`Units` supports pixels, physical units that use the image resolution, and a percentage of the shape's size:

```rust
use abra::abra_core::units::Units;

remover(RemoverShape::circle(12))
  .with_distance(Units::Pixels(6))
  .with_position((338, 1694))
  .apply(&mut image);

// 50% of the circle's radius.
remover(RemoverShape::circle(12))
  .with_distance(Units::Percent(50))
  .with_position((613, 1767))
  .apply(&mut image);
```

For a circle the percentage is of the radius. For other shapes it is of half the shorter side.

### Hardness

Hardness controls how the repair fades in from the edge of the shape. At `1.0` the repair reaches the edge with a hard, anti-aliased boundary. Lower values soften the inside edge, so pixels near the edge keep more of the original. The middle of the shape is always fully repaired.

Because the repair already matches the surrounding pixels at the edge, a soft edge is not needed to hide a seam. Use a low hardness when the shape is larger than the blemish, and `1.0` when the shape should be cleared completely. If part of the blemish is left near the edge, raise the hardness or make the shape a little larger.

## Reusing a stamp

A remover is a template. Build it once, then clone it and add the positions for each edit:

```rust
let stamp = remover(RemoverShape::circle(6)).with_distance(Units::Pixels(4));

stamp.clone().with_position((120, 80)).with_position((300, 210)).apply(&mut image);
stamp.clone().with_position((640, 512)).apply(&mut other_image);
```

A clone keeps everything the remover has, including its positions, and `with_position` and `with_path` add to them rather than replace them. Keep the template free of positions, as `stamp` is above, and add the positions to each clone. If a remover that already has positions is cloned and applied, it heals at those earlier positions again as well as at the new ones.

## How it works

For each position, the remover:

1. Rasterizes the shape into an anti-aliased mask over the part of the image it touches, clipped to the image bounds.
2. Finds the ring of pixels around the mask, out to the distance.
3. Searches nearby for the offset whose ring looks most like this one. It never borrows from the blemish itself, and ignores an overall brightness or color shift. If no place fits, the shape is filled smoothly from the ring alone.
4. Solves for pixels inside the shape whose detail comes from the borrowed patch and whose edge matches the surrounding pixels exactly.
5. Blends the result over the original using the mask and hardness.

Only color is changed. The alpha channel is left as it was.

## Notes

- Shapes that fall off the image are clipped, and a shape that misses the image entirely does nothing.
- A stroke borrows one patch for its whole length, so a path needs room in the image for a patch as long as the stroke. A stroke that spans nearly the whole image falls back to the smooth fill. Split it into several paths to borrow a patch for each.
- Very large shapes are slow, because the fill is solved iteratively over every pixel inside the shape.
- The remover suits blemishes, spots, and scratches on surfaces with a consistent texture. It does not rebuild complex structure, such as a shape crossing a strong edge.
