---
title: Paths
order: 3
outline: deep
---

# Paths

A `Path` is an open geometric shape made from a starting point and line or Bezier segments. Use paths for drawing, strokes, gradient directions, motion-like sampling, and other operations that do not require a filled interior.

For a closed region, use [`Area`](./areas).

## Creating paths

Create an empty path and build it with chainable methods:

```rust
use abra::abra_core::Path;

let mut path = Path::new();
path.move_to((20, 40))
  .line_to((120, 40))
  .quad_to((160, 80), (220, 40))
  .cubic_to((250, 10), (290, 70), (320, 40));
```

Convenience constructors cover common paths:

```rust
use abra::abra_core::Area;

let line = Path::line((0, 0), (320, 180));
let rectangle: Path = Area::rect((10, 10), (300, 160)).into();
let ellipse: Path = Area::ellipse((160, 90), (280, 160)).into();
```

Closed outlines such as rectangles and ellipses are built as an [`Area`](./areas) and converted into a `Path` when an open path is needed.

## Segments

A path contains `Segment` values:

- `Segment::Line { to }`
- `Segment::Quadratic { ctrl, to }`
- `Segment::Cubic { ctrl1, ctrl2, to }`

Inspect its structure with `start`, `end`, `segments`, and `points`:

```rust
let start = path.start();
let end = path.end();
let segments = path.segments();
let points = path.points();
```

`points` includes the start point and segment endpoints. Control points are available through the corresponding `Segment` variant.

## Sampling a path

Sample a point along the whole path with a normalized parameter:

```rust
let midpoint = path.point_at(0.5);
let segment_point = path.point_at_segment(1, 0.25);
```

`point_at` distributes the parameter across segments uniformly; it is not an arc-length parameterization. Values are clamped to `0.0..=1.0`.

## Flattening curves

Raster operations often need a polyline approximation. `flatten` converts curves into points using a tolerance:

```rust
let polyline = path.flatten(0.5);
```

Smaller tolerances produce more points and a closer approximation. Larger tolerances are faster but less precise.

## Measurements and bounds

```rust
let length = path.length();
let bounds = path.bounds(); // a Rect
let (min_x, min_y, max_x, max_y) = bounds.edges::<f32>();
let closest = path.closest_time(100.0, 60.0);
```

`closest_time` returns the normalized path position nearest to the supplied coordinates.

## Viewports

Map a path from an abstract coordinate system into a viewport, or fit it from its own bounds:

```rust
use abra::abra_core::{AspectRatio, Rect};

let design = Rect::new((0, 0), (320, 180));
let rendered = path.transform_to_viewport(&design, (640, 360), AspectRatio::meet());
let fitted = path.fit((640, 360), AspectRatio::meet());
```

See [Rectangles and viewports](./rect) for `Meet`, `Slice`, stretching, and alignment.

## Stroke a path

Expand an open path into an outline path with `stroke`:

```rust
use abra::abra_core::{LineCap, LineJoin};

let outline = path.stroke(12.0).with_join(LineJoin::Round).with_cap(LineCap::Round).to_path();
```

See [Strokes](./strokes) for cap and join styles.
