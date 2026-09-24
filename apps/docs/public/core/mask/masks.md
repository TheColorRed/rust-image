---
title: Masks
order: 1
outline: deep
---

# Masks

A `Mask` is a grayscale image that controls visibility or effect strength. White (`255`) is fully visible, black (`0`) is fully transparent, and gray values blend proportionally.

## Create a mask

Create a mask from an existing image:

```rust
use abra::mask::prelude::Mask;
use abra::prelude::*;

let source = Image::read("assets/mask.png")?;
let mask = Mask::from_image(source);
```

`Mask::new_from_image` creates a white mask with the same dimensions as a source image:

```rust
let source = Image::read("assets/photo.jpg")?;
let mask = Mask::new_from_image(&source);
```

Use a grayscale image for predictable mask behavior. RGBA mask images are converted to grayscale when their values are used as alpha controls.

## Draw an area into a mask

`draw_area` converts the supplied color to grayscale, fills the area, and composites it into the mask:

```rust
use abra::abra_core::{Area, Color};
use abra::mask::prelude::Mask;
use abra::prelude::*;

let base = Image::new(640, 360);
let mut mask = Mask::new_from_image(&base);
let area = Area::circle((320, 180), 120.0);

mask.draw_area(&area, Color::from_rgba(255, 255, 255, 255), None);
```

The optional position accepts a tuple, `PointF`, or `None` and defaults to `(0, 0)`.

Use gray to create partial effect strength:

```rust
mask.draw_area(
  &Area::rect((40, 40), (240, 140)),
  Color::from_rgba(128, 128, 128, 255),
  (20, 20),
);
```

## Apply a mask to an image

Apply the mask directly to an image to replace its alpha channel:

```rust
let mut image = Image::read("assets/photo.jpg")?;
mask.apply_to_image(&mut image);
image.write("out/masked.png", None)?;
```

The mask dimensions must match the image dimensions. Use a temporary canvas or resize the mask first when positioning or scaling is required.

For raw pixel buffers, use `apply_mask_to_pixels_rgba`:

```rust
use abra::mask::prelude::apply_mask_to_pixels_rgba;

let mut pixels = vec![255u8; 320 * 180 * 4];
let mask_bytes = vec![200u8; 320 * 180];
apply_mask_to_pixels_rgba(&mut pixels, &mask_bytes);
```

The mask input may contain one grayscale byte per pixel or four RGBA bytes per pixel. Invalid lengths panic because the dimensions cannot be inferred safely.

## Use a mask with an adjustment or filter

Pass a `Mask` through [`ApplyOptions`](../apply-options) when the operation should be modulated rather than immediately changing image alpha:

```rust
use abra::adjustments::prelude::color;
use abra::options::prelude::ApplyOptions;

let options = ApplyOptions::new().with_mask(mask);
color::grayscale().with_options(options).apply(&mut image);
```

This keeps the source image's alpha behavior separate from the operation's per-pixel strength.

## Helpers

`mask_value_to_alpha` maps a mask value directly to alpha. `rgba_to_gray` calculates grayscale using the ITU-R BT.601 approximation and ignores input alpha:

```rust
use abra::mask::prelude::{mask_value_to_alpha, rgba_to_gray};

assert_eq!(mask_value_to_alpha(128), 128);
let gray = rgba_to_gray(&[255, 0, 0, 255]);
```

## API summary

| API                                       | Purpose                                         |
| ----------------------------------------- | ----------------------------------------------- |
| `Mask::from_image(image)`                 | Create a mask by consuming an image.            |
| `Mask::new_from_image(image)`             | Create a white mask matching image dimensions.  |
| `draw_area(area, color, position)`        | Paint grayscale mask values into an area.       |
| `image()`                                 | Access the underlying mask image.               |
| `apply_to_image(image)`                   | Set an image's alpha from the mask.             |
| `apply_mask_to_image(image, bytes)`       | Apply grayscale or RGBA mask bytes to an image. |
| `apply_mask_to_pixels_rgba(pixels, mask)` | Apply mask bytes to an RGBA buffer.             |
| `rgba_to_gray(rgba)`                      | Convert one RGBA pixel to grayscale.            |
