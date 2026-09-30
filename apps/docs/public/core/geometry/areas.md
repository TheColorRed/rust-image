---
title: Areas
order: 4
outline: deep
---

# Areas

An `Area` represents a closed shape made from lines and curves. Areas are the geometry type used for fills, containment tests, effects, and region-based drawing.

## Creating areas

Create common areas directly:

```rust
use abra::abra_core::{Area, Image};

let empty = Area::new();
let rectangle = Area::rect((20, 20), (280, 160));
let circle = Area::circle((160, 100), 80.0);
let ellipse = Area::ellipse((160, 100), (280, 140));
let from_image = Area::new_from_image(&Image::new(320, 200));
```

Create a polygonal area from points:

```rust
let triangle = Area::from_points(&[
  [160.0, 10.0],
  [310.0, 190.0],
  [10.0, 190.0],
]);
```

An area is built on top of a `Path`, so you can also construct one incrementally:

```rust
let mut shape = Area::new();
shape.move_to((20, 20))
  .line_to((300, 20))
  .quad_to((320, 100), (300, 180))
  .line_to((20, 180));
```

## Filling and feathering

Pass an area to `fill` to rasterize it:

```rust
use abra::abra_core::Color;
use abra::drawing::prelude::fill;

let soft_circle = Area::circle((160, 100), 80.0).with_feather(12);
let image = fill(soft_circle, Color::from_rgba(40, 120, 220, 255)).to_image();
```

`with_feather` sets an edge feather radius in pixels. Read it with `feather`.

## Containment

Test whether a coordinate lies inside the closed area:

```rust
let area = Area::rect((0, 0), (100, 80));
assert!(area.contains((40, 30)));
assert!(!area.contains((120, 30)));
```

Containment uses a ray-casting test over the flattened path. `contains` flattens the outline on every call; to test many points, flatten once and use `polygon_contains`:

```rust
use abra::abra_core::{PointF, polygon_contains};

let outline = area.flatten(0.5);
let inside = polygon_contains(&outline, PointF::new(40.0, 30.0));
```

## Building and inspecting geometry

An area dereferences to its outline `Path`, so every path method can be called on it directly:

```rust
let start = area.start();
let end = area.end();
let segments = area.segments();
let points = area.points();
let sampled = area.point_at(0.5);
let length = area.length();
let bounds = area.bounds(); // a Rect
```

Use `flatten` when a raster or polygon operation needs discrete points:

```rust
let curve_points = area.flatten(0.5);
```

## Fitting and transforming areas

`fit` scales an area from its own bounds into a target size. The `AspectRatio` decides how differing proportions are handled, and the feather is kept:

```rust
use abra::abra_core::{AspectRatio, Size};

let fitted = area.fit(Size::new(800, 600), AspectRatio::meet());
let stretched = area.fit((800, 600), AspectRatio::none());
let covered = area.fit((800, 600), AspectRatio::slice());
```

Use `transform_to_viewport` when an explicit coordinate system is needed. See [Rectangles and viewports](./rect).

## Stroking an area

`Area::stroke` creates a closed area around the boundary, which can then be filled:

```rust
use abra::abra_core::{Color, LineJoin};
use abra::drawing::prelude::fill;

let border = area.stroke(8.0).with_join(LineJoin::Round).to_area();
let image = fill(border, Color::black()).to_image();
```

See [Strokes](./strokes) for line joins and caps.
