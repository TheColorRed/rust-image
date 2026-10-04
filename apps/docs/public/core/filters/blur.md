---
title: Blur filters
order: 2
outline: deep
---

# Blur filters

Blur filters soften image detail by sampling neighboring pixels. Most blur functions mutate the image in place. See [Apply options](../apply-options) for regional processing.

## Gaussian blur

`gaussian_blur` builds a separable Gaussian blur operation. The radius controls the amount of blur:

```rust
use abra::filters::prelude::blur::gaussian_blur;
use abra::prelude::*;

let mut image = Image::read("assets/photo.jpg")?;
gaussian_blur(12).apply(&mut image);
image.write("out/gaussian.png", None)?;
```

A radius of `0` is a no-op. Large radii and sufficiently large areas may use downsampling for performance.

## Box blur

`box_blur` averages pixels inside a square neighborhood and is applied through the builder API:

```rust
use abra::filters::prelude::blur::box_blur;

box_blur(8).apply(&mut image);
```

The radius is converted to a non-negative integer.

## General blur

Use `blur` for the module's general blur operation:

```rust
use abra::filters::prelude::blur::blur;

blur().apply(&mut image);
```

## Motion blur

`motion_blur` blurs along an angle and distance:

```rust
use abra::filters::prelude::blur::motion_blur;

motion_blur(45.0, 24).apply(&mut image);
```

The angle is expressed in degrees and distance in pixels.

## Surface blur

`surface_blur` smooths within a threshold, preserving stronger edges:

```rust
use abra::filters::prelude::blur::surface_blur;

surface_blur(8, 20).apply(&mut image);
```

The arguments are radius and threshold. It runs on the GPU when one is available, and gives the same pixels as the CPU.

For a large photo, `with_step` spaces the pixels the blur averages apart, so the same number of pixels reaches that many times as far. The cost of the blur depends on how many pixels it averages, so this blurs a large photo as widely as a small one would be, for the same cost:

```rust
// Looks at 10 pixels each way, 3 pixels apart, so it reaches 30 pixels.
surface_blur(10, 20).with_step(3).apply(&mut image);
```

## Focus blur

`focus_blur` keeps a focus geometry sharper while applying a Gaussian or lens blur outside it:

```rust
use abra::filters::prelude::blur::{ApertureShape, FocusGeometry, FocusShape, focus_blur, lens_blur};

focus_blur().apply(&mut image);

focus_blur()
  .with_shape(FocusShape::Horizontal)
  .with_blur(lens_blur(12).with_shape(ApertureShape::Octagon))
  .with_geometry(FocusGeometry::new().with_center_y(0.6).with_radius(0.4).with_rotation(-10.0))
  .apply(&mut image);
```

With no settings, the focus is a circle in the center and the rest of the image gets `gaussian_blur(25)`.

| `FocusBlur` method               | Default             | Behavior                                  |
| -------------------------------- | ------------------- | ----------------------------------------- |
| `with_shape(FocusShape)`         | `Circle`            | `Circle`, `Square`, `Diamond`, `Horizontal` (band), or `Vertical` (band). |
| `with_blur(blur)`                | `gaussian_blur(25)` | A `gaussian_blur(..)` or `lens_blur(..)` builder, configured as usual. |
| `with_geometry(FocusGeometry)`   | see below           | Where the focus sits and how its edge fades. |

Distances in `FocusGeometry` are normalized so `1.0` is half of the image's shorter side.

| `FocusGeometry` method   | Default | Behavior                                                                 |
| ------------------------ | ------- | ------------------------------------------------------------------------ |
| `with_center_x(f32)`     | `0.5`   | Center as a fraction of the width, `0.0..=1.0`.                          |
| `with_center_y(f32)`     | `0.5`   | Center as a fraction of the height, `0.0..=1.0`.                         |
| `with_radius(f32)`       | `0.75`  | Distance where the blur reaches full strength.                           |
| `with_sharpness(f32)`    | `0.25`  | Fraction of the radius that stays fully sharp. `1.0` is a hard edge.     |
| `with_midpoint(f32)`     | `0.5`   | Where in the fade the blur reaches half strength. Lower blurs sooner.    |
| `with_aspect_ratio(f32)` | `0.0`   | Flattens the shape vertically. Ignored by the band shapes.               |
| `with_rotation(f32)`     | `0.0`   | Clockwise rotation in degrees, `-180.0..=180.0`.                         |

`with_options` still works: a mask is combined with the focus mask, and an area limits where the blur can land. Options set on the blur passed to `with_blur` are replaced by the focus mask.

## Lens blur

`lens_blur` simulates an aperture-shaped blur and optional bokeh details:

```rust
use abra::filters::prelude::blur::{ApertureShape, lens_blur};
use abra::filters::prelude::noise::NoiseDistribution;

lens_blur(8).apply(&mut image);

lens_blur(12)
  .with_shape(ApertureShape::Pentagon)
  .with_blade_curvature(0.2)
  .with_specular(1.5, 0.8)
  .with_noise(0.02, NoiseDistribution::Gaussian)
  .apply(&mut image);
```

The argument is the blur radius in pixels.

| Method                                | Default   | Behavior                                                                        |
| ------------------------------------- | --------- | ------------------------------------------------------------------------------- |
| `with_shape(ApertureShape)`           | `Hexagon` | `Triangle`, `Square`, `Pentagon`, `Hexagon`, `Heptagon`, or `Octagon`.          |
| `with_blade_curvature(f32)`           | `0.5`     | `0.0` gives a straight-sided polygon, `1.0` a circle.                           |
| `with_rotation(f32)`                  | `0.0`     | Clockwise rotation of the aperture in degrees.                                  |
| `with_specular(brightness, threshold)`| off       | Multiplies samples brighter than `threshold` (`0.0..=1.0`) by `brightness`.     |
| `with_noise(amount, distribution)`    | off       | Adds noise after blurring, to match the grain of the rest of the image.         |
| `with_samples(u32)`                   | `32`      | Samples per pixel. Higher is smoother but slower.                               |
