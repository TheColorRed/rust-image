---
title: Geometry Overview
order: 0
outline: deep
---

# Geometry Overview

Abra's geometry module provides the coordinate types and shapes used by drawing, filling, stroking, transforms, and viewbox rendering.

The central distinction is:

- `Path` is an open sequence of lines and curves.
- `Area` is a closed shape intended for fills, hit testing, and effects.
- `Point` and `PointF` represent integer and floating-point coordinates.
- `ViewBox` maps abstract geometry into a target viewport.

## A complete geometry workflow

Build a shape, convert it into a stroked or filled result, and render it with the drawing APIs:

```rust
use abra::abra_core::{Area, Color, Path};
use abra::drawing::prelude::{fill, LineCap, LineJoin};

let area = Area::rect((20, 20), (280, 160));
let outline = area.stroke(8.0).with_join(LineJoin::Round).to_area();
let outlined = fill(outline, Color::from_rgba(30, 100, 220, 255)).to_image();

let line = Path::line((0, 0), (320, 200));
let rounded_line = line.stroke(12.0).with_join(LineJoin::Round).with_cap(LineCap::Round).to_path();
let line_image = fill(rounded_line, Color::white()).to_image();
```

Use `Area` directly with `fill`, or use `Path` when you need an open line, a gradient direction, or a path to stroke.

## Coordinate types

`Point` stores integer coordinates and is useful for raster operations. `PointF` stores `f32` coordinates for curves and precise geometric calculations. `Size` stores floating-point width and height.

```rust
use abra::abra_core::{Point, PointF, Size};

let pixel = Point::new(24, 12);
let precise = PointF::new(24.5, 12.25);
let dimensions = Size::new(320, 180);
```

These types accept common tuples through `Into` conversions, so `(x, y)` and `(width, height)` can often be passed directly.

## Shapes

Use the built-in shapes for common closed geometry:

```rust
use abra::abra_core::{Heart, Polygon, Star};

let heart = Heart::new();
let triangle = Polygon::new(3);
let star = Star::new();
```

Built-in shapes are defined in a normalized coordinate space and return `Area` values. See [Shapes](./shapes).

## Viewport rendering

Define geometry in an abstract coordinate system, then map it to different output sizes with a `ViewBox`:

```rust
use abra::abra_core::{AspectRatio, Heart, ViewBox};

let heart = Heart::new();
let viewbox = ViewBox::new(0.0, 0.0, 100.0, 120.0);
let path = heart.transform_to_viewport(&viewbox, 500.0, 600.0, AspectRatio::meet());
```

See [ViewBox](./viewbox) for scaling, alignment, and aspect-ratio behavior.

## Geometry pages

| Page                       | Covers                                                               |
| -------------------------- | -------------------------------------------------------------------- |
| [Primitives](./primitives) | `Point`, `PointF`, `Size`, and line helpers.                         |
| [Paths](./paths)           | Lines, quadratic and cubic curves, sampling, flattening, and bounds. |
| [Areas](./areas)           | Closed shapes, containment, fitting, and area measurements.          |
| [Strokes](./strokes)       | Expanding paths and areas with cap and join styles.                  |
| [Shapes](./shapes)         | Heart, polygon, and star constructors.                               |
| [ViewBox](./viewbox)       | Resolution-independent coordinate mapping.                           |
