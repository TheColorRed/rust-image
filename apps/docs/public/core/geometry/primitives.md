---
title: Geometry primitives
order: 2
outline: deep
---

# Geometry primitives

Abra provides three small coordinate types for geometry: `Point`, `PointF`, and `Size`. The module also includes Bresenham line helpers for integer raster coordinates.

## Point

`Point` stores integer `x` and `y` coordinates:

```rust
use abra::abra_core::Point;

let point = Point::new(12, 24);
let (x, y) = point.dimensions();
assert_eq!(point.x(), x);
assert_eq!(point.y(), y);
```

Create a collection of points from values that can be converted into `Point`:

```rust
let points = Point::array(vec![(0, 0), (10, 20), (30, 10)]);
```

`Point` supports addition, scalar multiplication, and point-by-point multiplication. It converts to integer and floating-point tuples.

## PointF

`PointF` stores `f32` coordinates and is the preferred type for curves and precise geometry:

```rust
use abra::abra_core::PointF;

let point = PointF::new(12.5, 24.25);
let origin = PointF::zero();
let direction = (point - origin).normalize();
```

Useful vector operations include:

```rust
let length = point.length();
let squared = point.length_squared();
let distance = point.distance_to(PointF::new(20.0, 30.0));
let dot = point.dot(PointF::new(1.0, 0.0));
let cross = point.cross(PointF::new(1.0, 0.0));
let perpendicular = point.perpendicular();
let midpoint = point.lerp(PointF::zero(), 0.5);
```

`PointF` converts to and from common tuples and `Point`. Conversion to integer coordinates rounds the floating-point values.

## Size

`Size` stores floating-point width and height:

```rust
use abra::abra_core::Size;

let size = Size::new(320, 180);
let tuple: (f32, f32) = size.to_tuple();
```

It accepts `(f32, f32)`, `(u32, u32)`, and `(i32, i32)` tuples. Arithmetic with scalars or another `Size` supports layout calculations:

```rust
let doubled = size * 2.0;
let inset = size - 20.0;
let half = size / 2.0;
```

## Bresenham lines

The geometry module re-exports `bresenham` and `bresenham_from_points` for generating integer points along a raster line:

```rust
use abra::abra_core::{bresenham, Point};

let pixels: Vec<Point> = bresenham((0, 0), (8, 5));
```

Use these helpers when a raster operation needs every integer coordinate on a line. Use [`Path`](./paths) for vector lines and curves.

## Conversion guidance

| Type | Best for |
| --- | --- |
| `Point` | Integer pixels and raster coordinates. |
| `PointF` | Curves, transforms, vectors, and subpixel geometry. |
| `Size` | Width and height calculations. |
| `Path` | Open lines and curves. |
| `Area` | Closed regions for fills and hit testing. |
