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

Use `Painter::dab_brush` to paint one dab at an image coordinate:

```rust
use abra::drawing::prelude::Painter;
use abra::prelude::*;

let mut image = Image::new(640, 360);
Painter::new(&mut image).dab_brush(320.0, 180.0, &brush);
image.write("out/brush-dab.png", None)?;
```

## Stroke a path

Use `Painter::stroke_with_brush` for a continuous brush stroke:

```rust
use abra::abra_core::Path;
use abra::drawing::prelude::Painter;

let path = Path::line((40, 180), (600, 180));
Painter::new(&mut image).stroke_with_brush(&path, &brush);
```

Curved paths and brush gradients are supported through the same API.

### Taper or swell a stroke

The brush's size is how wide a stroke is, the same all the way along it. `with_width` sets how wide the stroke is along the path instead, with a `StrokeWidth` that sets the width at stops, where `0` is the start of the path and `1` is its end (see [Strokes](../geometry/strokes)):

```rust
use abra::abra_core::{StrokeWidth, WidthStop};
use abra::drawing::prelude::Painter;

// Thick at the start and a point at the end, like a brush lifted off the page.
let tapered = Brush::new().with_width(StrokeWidth::from_to(24.0, 0.0)).with_hardness(1.0);
Painter::new(&mut image).stroke_with_brush(&path, &tapered);

// Thin, swelling a third of the way along, then thinning to a point.
let inked = Brush::new().with_width(StrokeWidth::new(vec![
  WidthStop::new(3.0, 0.0),
  WidthStop::new(20.0, 0.35),
  WidthStop::new(0.0, 1.0),
]));
```

The soft edge follows the width: at a thin part of the stroke the fade is over a thin distance, so a taper stays a point. `with_width` takes a number too, which is the same as `with_size` for a stroke, and `with_size` called after it replaces it. A dab and `fill_area_with_brush` have no line to run along, so they use the size. A line through points (see below) reads the width along its points. `width()` gives the width a stroke will use: what was set, or the size all the way.

### Fade a stroke

`with_opacity` takes a number, which is that opaque the whole way along a stroke (`1.0`, solid, is the default), or a `StrokeOpacity`, which sets the opacity at stops along the stroke exactly as `StrokeWidth` sets the width: `0` is the start of the stroke and `1` is its end.

```rust
use abra::abra_core::{OpacityStop, StrokeEasing, StrokeOpacity};

// Solid at the start and clear at the end.
let fading = Brush::new().with_size(20).with_opacity(StrokeOpacity::from_to(1.0, 0.0));

// Clear, solid a third of the way along, then clear again, with the changes eased.
let glow = Brush::new().with_opacity(
  StrokeOpacity::new(vec![OpacityStop::new(0.0, 0.0), OpacityStop::new(1.0, 0.33), OpacityStop::new(0.0, 1.0)])
    .with_easing(StrokeEasing::Smooth),
);

// The same opacity the whole way.
let half = Brush::new().with_opacity(0.5);
```

Between two stops the opacity changes at a steady rate (`StrokeEasing::Linear`, the default) or eases (`StrokeEasing::Smooth`). Before the first stop and after the last it is that stop's, it is held to `0..=1`, and a `StrokeOpacity` with no stops is solid. `opacity_at(time)` reads it anywhere along the stroke and `constant_opacity()` gives the opacity when it never changes. Width and opacity are independent, so one stroke can taper and fade at once.

A dab, `fill_area_with_brush` and the stroke of a closed area have no start or end to run along, so they use the opacity at the start. `Painter::stroke_with_brush` and `stroke_overlay` read it along the line.

## Draw a line through points

`stroke_overlay` draws a brush along a list of points, such as the touches of a finger, and gives a transparent image to blend over a photo. It is for a line that is redrawn as it grows. `stroke_overlay_deferred` leaves the pixels to the GPU when there is one, so adding a point costs one shader run, and the CPU gives the same pixels:

```rust
use abra::drawing::prelude::{stroke_overlay, stroke_overlay_deferred};

let points = [(40.0, 180.0), (200.0, 120.0), (360.0, 200.0), (600.0, 150.0)];
let line = stroke_overlay(640, 360, &points, &brush);
```

The brush's size, hardness, opacity and solid color are used (a gradient or image fill draws in black), and so is its width, read along the points: `0` is the first point and `1` is the last, by length. A line of more than `MAX_STROKE_POINTS` (256) points is thinned evenly, keeping both ends. A width that tapers to a point follows the end of the line as it grows, since the end is always `1`.

## Fill an area with brush dabs

`Painter::fill_area_with_brush` covers a closed area using brush dabs:

```rust
use abra::abra_core::{Area, Color};
use abra::drawing::prelude::{Brush, Painter};

let area = Area::ellipse((320, 180), (400, 220));
let brush = Brush::new()
  .with_size(24)
  .with_color(Color::blue())
  .with_hardness(0.8);

Painter::new(&mut image).fill_area_with_brush(&area, &brush);
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
| `Painter::dab_brush(x, y, brush)` | Paint one brush dab. |
| `Painter::stroke_with_brush(path, brush)` | Paint a continuous path stroke. |
| `Painter::fill_area_with_brush(area, brush)` | Fill a closed area with brush dabs. |
