---
title: Detail filters
order: 5
outline: deep
---

# Detail filters

Detail filters either increase local edge definition or reduce small variations. The family includes sharpening, general smoothing, and skin-aware smoothing. See [Apply options](../apply-options) for regional processing.

## Sharpen

`sharpen` increases local detail:

```rust
use abra::filters::prelude::sharpen::sharpen;

sharpen().apply(&mut image);
```

## Smooth

`smooth` reduces local variation across an image or selected area:

```rust
use abra::filters::prelude::smooth::smooth;

smooth().apply(&mut image);
```

## Skin smoothing

`smooth_skin` detects skin-like colors in HSV space and applies smoothing through a generated mask:

```rust
use abra::filters::prelude::smooth::smooth_skin;

smooth_skin(0.65).apply(&mut image);
```

The amount is clamped to `0.0..=1.0`. Existing masks can be combined with the detected skin mask through the shared application options. The implementation uses a Gaussian blur for the selected skin regions.

## Choosing a detail filter

- Use `sharpen` to restore or emphasize local detail.
- Use `smooth` for general softening.
- Use `smooth_skin` when smoothing should be limited to detected skin tones.
- Use [Blur](./blur) for explicit blur kernels and focus effects.
