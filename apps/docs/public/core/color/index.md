---
title: Color Overview
order: 0
outline: deep
---

# Color Overview

Color documentation covers RGBA values, named colors, fills, gradients, and histogram analysis.

## Color guides

| Guide                          | Covers                                                                                               |
| ------------------------------ | ---------------------------------------------------------------------------------------------------- |
| [Color](./color)               | Create colors, convert color spaces, calculate luminance and contrast, and compute color statistics. |
| [Named colors](./named-colors) | Common opaque color constructors and their RGBA values.                                              |
| [Fill](./fill)                 | Solid, gradient, and image fills for areas and existing images.                                      |
| [Gradient](./gradient)         | Color stops, direction, sampling, presets, and overlays.                                             |
| [Histogram](./histogram)       | Channel counts, clipping, levels LUTs, medians, means, and percentiles.                              |

## A complete color workflow

```rust
use abra::abra_core::{Color, Gradient};
use abra::adjustments::prelude::color;
use abra::prelude::*;

let mut image = Image::read("assets/photo.jpg")?;
color::auto_tone().apply(&mut image);
color::gradient_map(Gradient::from_to(Color::black(), Color::orange())).apply(&mut image);
image.write("out/color-treated.png", None)?;
```

Use [Adjustments](../adjustments/) for image-wide color operations and [Geometry](../geometry/) when applying fills to custom shapes.
