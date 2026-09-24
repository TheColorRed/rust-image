---
title: Kernels and convolution
order: 7
outline: deep
---

# Kernels and convolution

A kernel is a flat matrix of weights used to combine a pixel with its neighbors. Abra's internal kernel helper applies a 3x3 kernel to every pixel and writes the result back to the image.

## Apply a custom kernel

The crate's internal `apply_kernel` helper expects nine weights in row-major order. It is used by built-in filters and is not currently exposed as a public application API:

```rust
let blur_kernel = [
  1.0 / 9.0, 1.0 / 9.0, 1.0 / 9.0,
  1.0 / 9.0, 1.0 / 9.0, 1.0 / 9.0,
  1.0 / 9.0, 1.0 / 9.0, 1.0 / 9.0,
];

// Internal filter code calls: apply_kernel(&mut image, &blur_kernel);
```

The implementation clamps each resulting channel to `0..=255`. Pixels outside the image bounds are skipped while accumulating neighboring values.

## Common kernels

A sharpening kernel:

```rust
let sharpen_kernel = [
   0.0, -1.0,  0.0,
  -1.0,  5.0, -1.0,
   0.0, -1.0,  0.0,
];
```

An edge kernel:

```rust
let edge_kernel = [
  -1.0, -1.0, -1.0,
  -1.0,  8.0, -1.0,
  -1.0, -1.0, -1.0,
];
```

Kernels can affect the alpha channel as well because the convolution loops over all four RGBA channels. Use weights carefully when preserving transparency matters.

## Relationship to built-in filters

The kernel helper is the low-level mechanism used by some filters. Prefer a named filter when one exists because it can provide specialized border handling, area processing, or optimized implementations:

- Use `gaussian_blur` or `box_blur` for blur.
- Use `sharpen` for standard detail enhancement.
- Use Sobel functions for horizontal or vertical edge gradients.
- Use `apply_kernel` for custom convolution behavior.

The helper is currently an internal module implementation detail, so availability through a public prelude may vary by crate version.
