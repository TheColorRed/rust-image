---
title: Adjustments Overview
order: 0
outline: deep
---

# Adjustments Overview

Abra adjustments mutate an image's pixels to change color, tone, contrast, exposure, opacity, or palette. They operate in place.

## Basic workflow

```rust
use abra::adjustments::prelude::*;
use abra::prelude::*;

let mut image = Image::read("assets/photo.jpg")?;
color::grayscale().apply(&mut image);
image.write("out/grayscale.png", None)?;
```

All adjustment builders apply to the complete image by default:

```rust
levels::contrast(20).apply(&mut image);
levels::exposure(1.0).apply(&mut image);
color::invert().apply(&mut image);
```

Builders can be reused across images because `apply` borrows the configured adjustment:

```rust
let exposure = levels::exposure(1.0);
exposure.apply(&mut first_image);
exposure.apply(&mut second_image);
```

## Adjustment families

| Guide                                 | Covers                                                                                |
| ------------------------------------- | ------------------------------------------------------------------------------------- |
| [Levels and color controls](./levels) | Brightness, contrast, exposure, saturation, vibrance, hue, and photo filters.         |
| [Color operations](./color)           | Grayscale, invert, threshold, posterize, opacity, auto tone/color, and gradient maps. |

## Important behavior

- Adjustments mutate the supplied image or image reference; they do not return a new image.
- RGB operations generally preserve the alpha channel.
- Values outside documented ranges are clamped by the implementation where applicable.
- The public `hue` function is currently a placeholder and does not modify pixels.

## Save the result

```rust
image.write("out/adjusted.png", None)?;
```

Use [Apply options](../apply-options), [Color](../color/color), [Histogram](../color/histogram), and [Fill](../color/fill) alongside adjustments when building complete color workflows.
