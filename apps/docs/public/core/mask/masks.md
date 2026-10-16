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

## Grow, shrink and refine a mask

`dilated` grows a mask and `eroded` shrinks it: each pixel takes the strongest (or weakest) value within a number of pixels, so the edge of a region moves out or in by that many pixels. The window is a square, and the edges of the mask are not eaten.

```rust
let wider = mask.dilated(3);
let tighter = mask.eroded(3);
```

A mask found by a model is rough, because the model looked at a small copy of the photo. `guided_filter` makes it follow the edges of the photo itself: where the photo has a real edge the mask changes there, and where the photo is flat the mask is smoothed. The result is the size of the photo, so a mask found on a small copy can be refined at full size.

```rust
use abra::filters::prelude::smooth::guided_filter;

// Look 8 pixels around each pixel; follow edges as faint as 0.0001 (colors count from 0 to 1).
let refined = guided_filter(&mask, &photo, 8, 0.0001);
```

A radius of a few pixels fixes an edge that is a few pixels off, and a larger `epsilon` (`0.01`) follows only strong edges. It smooths and places an edge, but it does not sharpen a very blurry one.

`guided_filter` counts all three colors of the photo and runs on the CPU. For a picture of a mask that should be refined on the GPU, use the `guided` effect, which looks at the photo's brightness only and gives the same pixels on the CPU and the GPU:

```rust
use abra::filters::prelude::smooth::guided;

// `mask_image` is a picture of the mask (white where it reaches); the red channel is read.
guided(&photo, 8, 0.0001).apply(&mut mask_image);
```

On the GPU it keeps its numbers in 32-bit float textures (16 bytes a pixel, two of them), so use a picture of the mask no larger than it needs to be. To grow or shrink a mask picture on the GPU, see [Grow and shrink](../filters/morphology).

## Pull out thin parts against a background

`Mask::from_color_mix` makes a mask of how much of each pixel is a foreground color rather than a background color, taking the pixel's color as a mix of the two. It is for thin parts such as hair, fur or smoke, where the pixels at the edge are part foreground and part background and a mask found by a model leaves them out or gets them wrong:

```rust
// `foreground` and `background` are pictures the size of the photo: the color the foreground has near each pixel and the color the
// background has there. `rough` is a mask to use where the two colors are too alike to tell (and where they are less than 0.1 apart).
let fine = Mask::from_color_mix(&photo, &foreground, &background, &rough, 0.1);
```

A pixel exactly the foreground color is `255`, exactly the background color is `0`, and halfway between is half. The two colors can differ at every pixel, so they are made from the sure foreground and the sure background of a rough mask: shrink the mask for the sure foreground and grow it (and invert it) for the sure background (`eroded` and `dilated`), keep only the photo's color inside each, and blur them outwards so they fill in under the uncertain band at the edge. The pictures must all be the same size, or it panics.

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
| `dilated(radius)` / `eroded(radius)`      | Move the edge of a mask out or in.              |
| `from_color_mix(photo, fg, bg, fallback, least)` | The share of a foreground color in each pixel. |
| `guided_filter(mask, photo, radius, eps)` | Make a mask follow the edges of a photo (color, CPU). |
| `guided(photo, radius, eps)`              | The same by brightness, as an effect on the CPU or GPU. |
