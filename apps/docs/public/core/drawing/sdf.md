---
title: SDF drawing helpers
order: 5
outline: deep
---

# SDF drawing helpers

The drawing crate exposes signed-distance helpers and optimized solid-color drawing functions. These are lower-level APIs; use [Brushes](./brushes) and [Painter](./painter) for most application workflows.

## Signed distance functions

`sdf_ellipse` returns an approximate signed distance from a point to an ellipse. `sdf_polygon` returns a negative distance for points inside a polygon and a positive distance outside it:

```rust
use abra::abra_core::PointF;
use abra::drawing::prelude::{sdf_ellipse, sdf_polygon};

let ellipse_distance = sdf_ellipse(100.0, 80.0, 60.0, 40.0, 100.0, 80.0);
let polygon_distance = sdf_polygon(
  &[
    PointF::new(0.0, 0.0),
    PointF::new(100.0, 0.0),
    PointF::new(50.0, 80.0),
  ],
  50.0,
  30.0,
);
```

Distances are approximate pixel-space values. The helpers are useful for custom coverage, procedural effects, and geometry tests.

## Draw solid areas and ellipse strokes

The optimized helpers accept a `Brush` and write directly into an image:

```rust
use abra::abra_core::{Area, Color, Image};
use abra::drawing::prelude::{draw_area_fill, draw_area_stroke, Brush};

let mut image = Image::new(320, 240);
let brush = Brush::new().with_size(12).with_color(Color::blue());
let area = Area::ellipse((160, 120), (90, 60));

draw_area_fill(&mut image, &area, &brush);
draw_area_stroke(&mut image, &area, &brush);
```

`draw_ellipse_stroke` is the specialized form when the ellipse center and radii are already known:

```rust
use abra::drawing::prelude::draw_ellipse_stroke;

draw_ellipse_stroke(&mut image, 160.0, 120.0, 90.0, 60.0, &brush);
```

These optimized helpers currently support solid brush fills. For gradients or image fills, use the general rasterization pipeline.

## API summary

| API                                                 | Purpose                                    |
| --------------------------------------------------- | ------------------------------------------ |
| `sdf_ellipse(cx, cy, rx, ry, x, y)`                 | Approximate signed distance to an ellipse. |
| `sdf_polygon(points, x, y)`                         | Signed distance to a closed polygon.       |
| `draw_area_fill(image, area, brush)`                | Draw a brush-filled area.                  |
| `draw_area_stroke(image, area, brush)`              | Draw an area outline.                      |
| `draw_ellipse_stroke(image, cx, cy, rx, ry, brush)` | Draw an optimized ellipse outline.         |
