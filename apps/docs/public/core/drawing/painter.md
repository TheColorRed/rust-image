---
title: Painter
order: 3
outline: deep
---

# Painter

`Painter` is a mutable drawing context for repeated operations on one `Image`. It wraps the target image and exposes brush dabs, brush strokes, and area fills as methods.

## Create a painter

```rust
use abra::drawing::prelude::Painter;
use abra::prelude::*;

let mut image = Image::new(640, 360);
let mut painter = Painter::new(&mut image);
```

Access the underlying image through `image()` when a drawing workflow needs direct image operations:

```rust
painter.image().save("out/painter.png", None);
```

## Paint repeated dabs

```rust
use abra::abra_core::Color;
use abra::drawing::prelude::{Brush, Painter};

let brush = Brush::new()
  .with_size(24)
  .with_color(Color::orange())
  .with_hardness(0.65);

painter.dab_brush(120.0, 120.0, &brush);
painter.dab_brush(180.0, 150.0, &brush);
painter.dab_brush(240.0, 180.0, &brush);
```

## Stroke paths

```rust
use abra::abra_core::Path;

let path = Path::line((40, 240), (600, 240));
painter.stroke_with_brush(&path, &brush);
```

The painter converts the path into a stroked area and uses brush hardness and fill to shade the stroke.

## Fill areas with a brush

```rust
use abra::abra_core::Area;

let area = Area::circle((320, 160), 100.0);
painter.fill_area_with_brush(&area, &brush);
```

## Convenience functions

For one-off operations, use the free functions documented in [Brushes](./brushes): `paint_with_brush`, `stroke_with_brush`, and `fill_area_with_brush`. Use `Painter` when several operations target the same image and a context makes the workflow clearer.

## Rasterization behavior

Drawing uses polygon coverage, shaders, anti-aliasing samples, and source-over compositing. Large brushes may use a lower anti-aliasing level for performance, while smaller brushes use more samples for smoother edges.
