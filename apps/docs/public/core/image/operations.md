---
title: Image operations
order: 1
outline: deep
---

# Image operations

Use `Image` for raster construction, pixel access, dimensions, and RGBA buffer updates.

## Create an image

Create a blank RGBA image or load one from a file:

```rust
use abra::prelude::*;

let blank = Image::new(640, 360);
let photo = Image::read("assets/photo.jpg")?;
```

Create an image from an existing RGBA buffer when the buffer length is `width * height * 4`:

```rust
use abra::abra_core::{Channels, Image};

let pixels = vec![255u8; 320 * 200 * 4];
let image = Image::new_from_pixels(320, 200, pixels, Channels::RGBA);
```

## Inspect dimensions and pixels

```rust
let (width, height): (u32, u32) = image.dimensions();
let pixel = image.get_pixel(10, 20);
```

`get_pixel` returns an optional RGBA tuple because coordinates outside the image have no pixel.

Set one pixel with an RGBA tuple:

```rust
let mut image = Image::new(320, 200);
image.set_pixel(10, 20, (255, 80, 40, 255));
```

## Read and update RGBA data

Use `rgba` for a read-only contiguous RGBA slice and `to_rgba_vec` when an owned buffer is more convenient:

```rust
let bytes: &[u8] = image.rgba();
let owned: Vec<u8> = image.to_rgba_vec();
```

Replace all pixels with an owned RGBA buffer:

```rust
let mut pixels = image.to_rgba_vec();
pixels[3] = 200;
image.set_rgba_owned(pixels);
```

Use `set_rgba` when borrowing the input buffer:

```rust
let pixels = vec![0u8; 320 * 200 * 4];
image.set_rgba(&pixels);
```

For per-pixel processing, use the mutation helpers exposed by the prelude:

```rust
image.mut_pixels(|mut pixel| {
  pixel[0] = pixel[0].saturating_add(10);
});
```

The callback receives the four channels in RGBA order. Keep the alpha channel unchanged when an operation is intended to affect color only.

## Copy and replace image data

`clone` creates an independent image value with shared storage where supported by the underlying image implementation. A later mutation uses copy-on-write semantics:

```rust
let original = Image::read("assets/photo.jpg")?;
let mut edited = original.clone();
edited.set_pixel(0, 0, (0, 0, 0, 255));
```

Use `set_new_pixels` when replacing both the RGBA buffer and dimensions:

```rust
let pixels = vec![255u8; 100 * 100 * 4];
image.set_new_pixels(&pixels, 100, 100);
```

## Channels and alpha

Abra images use four bytes per pixel in red, green, blue, alpha order. Alpha values range from `0` (transparent) to `255` (opaque). The [`Masks`](../mask/) and [`Apply options`](../apply-options) guides describe reusable alpha and effect-strength workflows.

## API summary

| API                                         | Purpose                                         |
| ------------------------------------------- | ----------------------------------------------- |
| `Image::new(width, height)`                 | Create a blank image.                           |
| `Image::read(path)`                         | Fallibly load an image through extension-based I/O. |
| `Image::new_from_pixels(...)`               | Create an image from raw channel data.          |
| `dimensions()`                              | Read width and height.                          |
| `get_pixel(x, y)` / `set_pixel(x, y, rgba)` | Read or write one pixel.                        |
| `rgba()` / `to_rgba_vec()`                  | Read the RGBA buffer.                           |
| `set_rgba()` / `set_rgba_owned()`           | Replace RGBA data.                              |
| `set_new_pixels()`                          | Replace dimensions and pixel data.              |
| `mut_pixels()`                              | Mutate pixels through a callback.               |
| `save(path, options)`                       | Write an image; see [Loading and saving](./io). |
