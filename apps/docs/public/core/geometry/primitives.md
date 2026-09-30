---
title: Geometry primitives
order: 2
outline: deep
---

# Geometry primitives

Abra provides two small value types for geometry, `PointF` and `Size`, plus [`Rect`](./rect) for bounds. The module also includes a Bresenham line helper for integer raster coordinates.

## PointF

`PointF` stores `f32` coordinates. It is the one point type used across geometry, drawing, and compositing:

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

`PointF` converts from any `(x, y)` tuple of numbers, so APIs that take `impl Into<PointF>` accept `(10, 20)` directly. It converts to `(f32, f32)`, `(f64, f64)`, and, rounding to the nearest pixel, `(i32, i32)` and `(u32, u32)`.

## Size

`Size` stores floating-point width and height:

```rust
use abra::abra_core::Size;

let size = Size::new(320, 180);
let tuple: (u32, u32) = size.to_tuple();
```

It converts from any `(width, height)` tuple of numbers. Arithmetic with scalars or another `Size` supports layout calculations:

```rust
let doubled = size * 2.0;
let inset = size - 20.0;
let half = size / 2.0;
let per_pixel = 1.0 / size; // (1 / width, 1 / height)
```

`rotated_bounds(degrees)` is the canvas a rotated rectangle of this size needs, and `inscribed_after_rotation(degrees, aspect)` is the largest upright rectangle inside the rotation.

## Bresenham lines

`bresenham` returns every integer pixel on the straight line between two points, including both ends:

```rust
use abra::abra_core::bresenham;

let pixels: Vec<(i32, i32)> = bresenham((0, 0), (8, 5));
```

Use it when a raster operation needs every pixel on a line. Use [`Path`](./paths) for vector lines and curves.

## Conversion guidance

| Type     | Best for                                             |
| -------- | ---------------------------------------------------- |
| `PointF` | Positions, vectors, transforms, and pixel offsets.   |
| `Size`   | Width and height calculations.                       |
| `Rect`   | Bounds, clipping, and coordinate systems.            |
| `Path`   | Open lines and curves.                               |
| `Area`   | Closed regions for fills and hit testing.            |
