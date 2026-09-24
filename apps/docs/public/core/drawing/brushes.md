---
title: Brushes
order: 2
outline: deep
---

# Brushes

A `Brush` combines a size, closed shape, fill, hardness, and opacity. Use it for individual dabs, continuous path strokes, or filling an area with repeated brush coverage.

## Configure a brush

```rust
use abra::abra_core::{Color, Fill};
use abra::drawing::prelude::{Area, Brush};

let brush = Brush::new()
  .with_size(20)
  .with_area(Area::circle((0, 0), 10.0))
  .with_color(Fill::Solid(Color::red().into()))
  .with_hardness(0.75)
  .with_opacity(0.9);
```

Defaults are size `5`, a circular area with radius `5`, black color, hardness `0.0`, and opacity `1.0`.

Hardness and opacity are clamped to `0.0..=1.0`:

- Hardness `0.0` produces a soft falloff.
- Hardness `1.0` produces a sharp edge.
- Opacity `0.0` is transparent.
- Opacity `1.0` is fully opaque.

The brush fill can be a solid color, gradient, or image fill.

## Paint a dab

Use `paint_with_brush` to paint one dab at an image coordinate:

```rust
use abra::drawing::prelude::paint_with_brush;
use abra::prelude::*;

let mut image = Image::new(640, 360);
paint_with_brush(&mut image, 320.0, 180.0, &brush);
image.write("out/brush-dab.png", None)?;
```

## Stroke a path

Use `stroke_with_brush` for a continuous brush stroke:

```rust
use abra::abra_core::Path;
use abra::drawing::prelude::stroke_with_brush;

let path = Path::line((40, 180), (600, 180));
stroke_with_brush(&mut image, &path, &brush);
```

Curved paths and brush gradients are supported through the same API.

## Fill an area with brush dabs

`fill_area_with_brush` covers a closed area using brush dabs:

```rust
use abra::abra_core::{Area, Color};
use abra::drawing::prelude::{fill_area_with_brush, Brush};

let area = Area::ellipse((320, 180), (400, 220));
let brush = Brush::new()
  .with_size(24)
  .with_color(Color::blue())
  .with_hardness(0.8);

fill_area_with_brush(&mut image, &area, &brush);
```

The implementation samples dab centers across the area's bounds and uses coverage to keep paint within the shape.

## Brush properties

Read the configured values when building tools or UI controls:

```rust
let size = brush.size();
let shape = brush.area();
let fill = brush.color();
let hardness = brush.hardness();
let opacity = brush.opacity();
```

## API summary

| API | Purpose |
| --- | --- |
| `Brush::new()` | Create a brush with default values. |
| `with_size(size)` | Set brush diameter/working size. |
| `with_area(area)` | Set the brush shape. |
| `with_color(fill)` | Set a solid, gradient, or image fill. |
| `with_hardness(value)` | Set edge falloff from soft to hard. |
| `with_opacity(value)` | Set brush alpha strength. |
| `paint_with_brush(image, x, y, brush)` | Paint one brush dab. |
| `stroke_with_brush(image, path, brush)` | Paint a continuous path stroke. |
| `fill_area_with_brush(image, area, brush)` | Fill a closed area with brush dabs. |
