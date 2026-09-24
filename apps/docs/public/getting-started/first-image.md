---
title: First image
order: 3
outline: deep
---

# First image

This guide covers the basic `Image` lifecycle: create or load pixels, inspect dimensions, transform the image, and save it.

## Create an image

Create a solid image from dimensions and a color:

```rust
use abra::prelude::*;
use abra::abra_core::{FlipAxis, ResizeTarget};

fn main() {
  let image = Image::new_from_color(640, 360, Color::from_hex(0x203040));
  image.write("out/solid.png", None)?;
}
```

Create a transparent image when pixels will be drawn later:

```rust
let image = Image::new(640, 360);
```

## Load an image

Load an image from a path with `Image::read`:

```rust
use abra::prelude::*;

fn main() {
  let image = Image::read("assets/photo.jpg")?;
  image.write("out/copy.png", None)?;
}
```

The path is resolved by the running process. When using Cargo, run from the project directory or use paths that are valid from the current working directory.

## Inspect dimensions and pixels

Read dimensions by specifying the desired numeric type:

```rust
let (width, height): (u32, u32) = image.dimensions();
```

Read a pixel when you need to inspect RGBA output:

```rust
if let Some((r, g, b, a)) = image.get_pixel(10, 10) {
  println!("rgba=({}, {}, {}, {})", r, g, b, a);
}
```

Use `rgba()` when the complete pixel buffer is needed for analysis or another operation.

## Transform an image

Abra exposes resize, crop, and rotate operations through traits in the prelude:

```rust
use abra::prelude::*;

let mut image = Image::read("assets/photo.jpg")?;
image.resize(ResizeTarget::FitWidth(1200), None);
image.crop(0, 0, 1200, 800);
image.rotate(2.0, None);
flip(FlipAxis::Horizontal).apply(&mut image);
image.write("out/transformed.png", None)?;
```

The optional transform algorithm can be passed instead of `None` when a specific resize or rotation strategy is required.

## Draw and fill

Create an `Area`, fill it, and draw the generated image onto a canvas image:

```rust
use abra::abra_core::{Area, Color};
use abra::drawing::prelude::fill;

let mut image = Image::new(640, 360);
let area = Area::circle((320, 180), 120.0);
let circle = fill(area, Color::from_rgba(220, 80, 100, 255)).to_image();
image.draw_image_at(&circle, (0, 0));

// Or draw the fill straight into the image, where the area is.
fill(Area::rect((20, 20), (160, 80)), Color::from_rgba(40, 140, 220, 220)).apply(&mut image);
```

See [Fill](../core/color/fill) and [Geometry](../core/geometry/) for shape and fill details.

## Save output

Save the current image by providing a path and optional writer settings:

```rust
image.write("out/result.png", None)?;
```

Use a file extension supported by the image writer. Keep generated files in an ignored `out` directory during development.
