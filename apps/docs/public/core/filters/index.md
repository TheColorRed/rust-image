---
title: Filters Overview
order: 0
outline: deep
---

# Filters Overview

Filters modify image detail, texture, edges, or spatial structure. The filters crate is organized by effect family and works with Abra `Image` values.

## Filter families

The current filter implementation includes these modules:

| Family  | Purpose                                                        |
| ------- | -------------------------------------------------------------- |
| Blur    | Soften detail with blur kernels and specialized blur effects.  |
| Distort | Transform or displace image coordinates.                       |
| Edges   | Detect or emphasize image edges.                               |
| Noise   | Add or process image noise.                                    |
| Sharpen | Increase local detail and edge definition.                     |
| Smooth  | Reduce local variation while preserving the overall image.     |
| Kernel  | Apply a custom convolution kernel.                             |
| Sobel   | Detect horizontal and vertical gradients with Sobel operators. |

## Filter guides

| Guide                | Covers                                                         |
| -------------------- | -------------------------------------------------------------- |
| [Blur](./blur)       | Gaussian, box, motion, surface, focus, lens, and general blur. |
| [Distort](./distort) | Pinch and ripple displacement effects.                         |
| [Noise](./noise)     | Noise generation, median filtering, and despeckling.           |
| [Detail](./detail)   | Sharpening, smoothing, and skin smoothing.                     |
| [Edges](./edges)     | Glowing edges and Sobel horizontal/vertical operators.         |
| [Kernels](./kernels) | Custom 3x3 convolution kernels and low-level processing.       |

## Typical filter workflow

Filters mutate an image or return a processed image according to the specific filter API. A common workflow is:

```rust
use abra::prelude::*;

let mut image = Image::read("assets/photo.jpg")?;
// Apply a filter from the relevant filter module here.
image.write("out/filtered.png", None)?;
```

## Choosing between filters and adjustments

- Use [Adjustments](../adjustments/) for brightness, contrast, exposure, saturation, and color remapping.
- Use [Apply options](../apply-options) when a supported filter should target an area or mask.
- Use filters for spatial operations such as blur, sharpen, edge detection, noise, and distortion.
- Use [Canvas](../canvas/) when the effect should be composed as a separate layer or applied during rendering.

## Regional processing

Blur, distortion, noise, and smoothing APIs that accept an options argument can be restricted to an `Area` or modulated with a `Mask`. See [Apply options](../apply-options) for the shared builder, supported operation families, and the canonical regional-processing example.
