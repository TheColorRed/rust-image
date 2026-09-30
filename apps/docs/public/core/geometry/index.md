---
title: Geometry Overview
order: 0
outline: deep
---

# Geometry Overview

Abra's geometry module provides the coordinate types and shapes used by drawing, filling, stroking, transforms, and viewport rendering.

The central distinction is:

- `Path` is an open sequence of lines and curves.
- `Area` is a closed shape intended for fills, hit testing, and effects.
- `PointF` represents coordinates, and `Size` a width and height.
- `Rect` is an upright rectangle: every `bounds()` result, and the coordinate system geometry is mapped out of.

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

`PointF` stores `f32` coordinates. `Size` stores a floating-point width and height, and `Rect` an upright rectangle.

```rust
use abra::abra_core::{PointF, Rect, Size};

let point = PointF::new(24.5, 12.25);
let dimensions = Size::new(320, 180);
let frame = Rect::new((0, 0), dimensions);
```

These types accept common tuples through `Into` conversions, so `(x, y)` and `(width, height)` can often be passed directly.

## Shapes

Use the built-in shapes for common closed geometry:

```rust
use abra::abra_core::{Area, Shape};

let heart = Area::shape(Shape::Heart);
let triangle = Area::shape(Shape::Polygon(3));
let star = Area::shape(Shape::Star);
```

Shapes are drawn inside a `100 x 100` box and return `Area` values. See [Shapes](./shapes).

## Viewport rendering

Define geometry once, then fit it to different output sizes:

```rust
use abra::abra_core::{Area, AspectRatio, Shape};

let heart = Area::shape(Shape::Heart);
let large = heart.fit((500, 600), AspectRatio::meet());
let icon = heart.fit((32, 32), AspectRatio::meet());
```

See [Rectangles and viewports](./rect) for explicit coordinate systems, alignment, and aspect-ratio behavior.

## Geometry pages

| Page                       | Covers                                                               |
| -------------------------- | -------------------------------------------------------------------- |
| [Primitives](./primitives) | `PointF`, `Size`, and line helpers.                                  |
| [Paths](./paths)           | Lines, quadratic and cubic curves, sampling, flattening, and bounds. |
| [Areas](./areas)           | Closed shapes, containment, fitting, and area measurements.          |
| [Strokes](./strokes)       | Expanding paths and areas with cap and join styles.                  |
| [Shapes](./shapes)         | Heart, polygon, and star shapes.                                     |
| [Rectangles](./rect)       | `Rect`, clipping, and resolution-independent coordinate mapping.     |
