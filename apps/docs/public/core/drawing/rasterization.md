---
title: Rasterization primitives
order: 4
outline: deep
---

# Rasterization primitives

Most drawing code should use [`fill`](./fills), [`Brush`](./brushes), or [`Painter`](./painter). The lower-level rasterization API is useful when implementing a custom coverage shape, shader, or compositing rule.

## Rasterization pipeline

A `Rasterizer` combines four components:

1. A `CoverageMask` decides whether each sample lies inside the shape.
2. A `Shader` returns an RGBA color for a sample coordinate.
3. A `Compositor` combines the shaded sample with the destination pixel.
4. A `SampleGrid` controls anti-aliasing sample positions.

```rust
use abra::abra_core::Color;
use abra::drawing::prelude::*;

let coverage = PolygonCoverage::new(vec![
  (20.0, 20.0).into(),
  (220.0, 20.0).into(),
  (120.0, 180.0).into(),
]);
let shader = shader_from_fill(Color::red());
let compositor = SourceOverCompositor;
let samples = SampleGrid::from_aa_level(2);
let rasterizer = Rasterizer::new(&coverage, shader.as_ref(), &compositor, samples);

let mut image = abra::prelude::Image::new(240, 200);
rasterizer.rasterize(&mut image);
```

## Coverage masks

Implement `CoverageMask` for custom geometry:

```rust
struct Rectangle {
  min_x: f32,
  min_y: f32,
  max_x: f32,
  max_y: f32,
}

impl CoverageMask for Rectangle {
  fn contains(&self, x: f32, y: f32) -> bool {
    x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
  }

  fn bounds(&self) -> Option<(f32, f32, f32, f32)> {
    Some((self.min_x, self.min_y, self.max_x, self.max_y))
  }
}
```

`PolygonCoverage` provides a ready-made point-in-polygon implementation. Its vertices do not need to repeat the first point; the final edge is closed automatically.

## Shaders

Implement `Shader` to calculate an RGBA value at device coordinates:

```rust
struct Solid;

impl Shader for Solid {
  fn shade(&self, _x: f32, _y: f32) -> (u8, u8, u8, u8) {
    (20, 140, 240, 255)
  }
}
```

Use `shader_from_fill` for solid colors, gradients, and image fills. Use `shader_from_fill_with_path` when a gradient without an explicit direction needs a fallback path.

## Compositors and sampling

`SourceOverCompositor` applies standard alpha source-over compositing. Implement `Compositor` when a custom blend rule must receive source color, source alpha, coverage, and destination color.

`SampleGrid::from_aa_level` controls supersampling density. Higher levels improve edge smoothness at increased cost.

## API summary

| API | Purpose |
| --- | --- |
| `CoverageMask` | Define sample inclusion and optional bounds. |
| `PolygonCoverage::new(points)` | Create polygon coverage. |
| `Shader` | Calculate RGBA at a sample coordinate. |
| `shader_from_fill(fill)` | Build a shader from a `Fill`. |
| `shader_from_fill_with_path(fill, path)` | Build a fill shader with gradient fallback direction. |
| `Compositor` | Define source/destination compositing. |
| `SourceOverCompositor` | Use standard source-over alpha compositing. |
| `SampleGrid::from_aa_level(level)` | Create a supersampling grid. |
| `Rasterizer::new(...)` / `rasterize(image)` | Execute the pipeline. |
