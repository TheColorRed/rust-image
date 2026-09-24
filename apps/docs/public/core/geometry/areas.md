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

Containment uses a ray-casting test over the flattened path.

## Building and inspecting geometry

Area methods mirror the underlying path methods:

```rust
let start = area.start();
let end = area.end();
let segments = area.segments();
let points = area.points();
let sampled = area.point_at(0.5);
let length = area.length();
let bounds: (f32, f32, f32, f32) = area.bounds();
```

Use `flatten` or `to_points` when a raster or polygon operation needs discrete points:

```rust
let curve_points = area.flatten(0.5);
let pixel_points = area.to_points(0.5);
```

## Fitting and transforming areas

Fit or stretch an area into a target size:

```rust
use abra::abra_core::{AspectRatio, Size};

let fitted = area.fit(Size::new(800, 600));
let square = area.fit_square(400.0);
let aspect_fitted = area.fit_with_aspect(Size::new(800, 600), AspectRatio::meet());
let stretched = area.stretch((800, 600));
let covered = area.cover((800, 600));
```

Use `transform_to_viewport` when an explicit `ViewBox` and aspect-ratio policy are needed:

```rust
use abra::abra_core::ViewBox;

let viewbox = area.to_viewbox();
let rendered = area.transform_to_viewport(&viewbox, 400.0, 300.0, AspectRatio::meet());
```

## Stroking an area

`Area::stroke` creates a closed area around the boundary, which can then be filled:

```rust
use abra::abra_core::{Color, LineJoin};
use abra::drawing::prelude::fill;

let border = area.stroke(8.0).with_join(LineJoin::Round).to_area();
let image = fill(border, Color::black()).to_image();
```

See [Strokes](./strokes) for line joins and caps.
