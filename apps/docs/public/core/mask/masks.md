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

`opened` and `closed` clean a mask by shrinking and growing it in one go. `opened` shrinks it and then grows it back by the same amount, which takes away specks and thin spurs smaller than the window and keeps larger regions at their size. `closed` grows it and then shrinks it back, which fills holes and gaps smaller than the window and keeps the outside of a region where it was:

```rust
let clean = mask.opened(2); // stray dots gone
let whole = mask.closed(4); // the eyes of a face found as skin filled in
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

## Paint through a mask

`fill_image` paints a color, a gradient or a picture onto an image only where the mask is on. It is what Photoshop calls filling a layer clipped to a selection:

```rust
use abra::abra_core::{Color, Fill, Gradient, ImageRepeat};

skin.fill_image(&mut print, Color::from_rgb(255, 190, 150));       // a flat color
sky.fill_image(&mut print, Gradient::from_to(Color::blue(), Color::white())); // a gradient
shadows.fill_image(&mut print, Fill::image(dots).with_repeat(ImageRepeat::Both)); // a tiled picture
```

Where the mask is `255` the image gets the fill, where it is `0` the image is as it was, and in between the two are mixed. The fill is composited over the image, so a fill that is partly transparent shows what is under it. It is the same as filling an area the size of the image through the mask, which is how it was done before, without making the area:

```rust
// What fill_image does:
let everywhere = Area::rect((0.0, 0.0), (width as f32, height as f32));
fill(everywhere, color).with_mask(mask.clone()).apply(&mut image);
```

The fill starts at the top left corner of the image, so a gradient runs across the whole image and not only across the part that is on, and a picture starts at the corner (use `with_repeat` to tile it). The mask must be the size of the image, or it panics, as `multiply_alpha` does.

## Select by brightness

`from_brightness_range` makes a mask of the pixels of an image between two brightnesses, from `0` for black to `255` for white. It is the tone counterpart of `from_color_range`: use it to pick out the shadows, the highlights or the midtones, then limit an effect to them, or combine it with another mask to pick out the shadows of one person only. A `feather` of some levels eases the edge instead of cutting it.

```rust
use abra::mask::prelude::Mask;

let shadows = Mask::from_brightness_range(&photo, 0, 80, 20); // dark pixels, easing out over 20 levels
let midtones = Mask::from_brightness_range(&photo, 80, 180, 0);
```

The brightness of a pixel is the library's (`rgba_to_gray`, the ITU-R BT.601 weights, ignoring alpha). A `lightest` below `darkest` selects nothing.

`brightness_percentile` finds a level that suits what the mask covers, whatever the tones are. It gives the brightness at a share of the way through the covered pixels from the darkest to the lightest, so `0.5` is their median. Use it to set a threshold for the subject of a photo, for instance to turn the darkest third of a person black. The image must be the size of the mask:

```rust
let level = subject.brightness_percentile(&photo, 0.3, 127); // pixels of the mask above 127 are covered
let black = Mask::from_brightness_range(&photo, 0, level, 0).multiplied(&subject);
```

## Find the color of a part

`color_in` gives the one color that stands for the part of an image a mask covers: the average, median or most common value of those pixels. Use it to base a color on what is there, such as the color of a person's hair or shirt after a model has found them:

```rust
use abra::abra_core::ColorStat;

// The color of the hair, from the pixels well inside the mask (above 200), not its soft edge where the background has mixed in.
let hair_color = hair.color_in(&photo, ColorStat::Median, 200);
```

Each covered pixel counts the same, however strongly the mask covers it, so a threshold (or a mask grown in with `eroded`) keeps the edge out. It gives `None` when the mask covers nothing, and panics when the image and the mask are not the same size.

## Pull out thin parts against a background

`Mask::from_color_mix` makes a mask of how much of each pixel is a foreground color rather than a background color, taking the pixel's color as a mix of the two. It is for thin parts such as hair, fur or smoke, where the pixels at the edge are part foreground and part background and a mask found by a model leaves them out or gets them wrong:

```rust
// `foreground` and `background` are pictures the size of the photo: the color the foreground has near each pixel and the color the
// background has there. `rough` is a mask to use where the two colors are too alike to tell (and where they are less than 0.1 apart).
let fine = Mask::from_color_mix(&photo, &foreground, &background, &rough, 0.1);
```

A pixel exactly the foreground color is `255`, exactly the background color is `0`, and halfway between is half. The two colors can differ at every pixel, so they are made from the sure foreground and the sure background of a rough mask: shrink the mask for the sure foreground and grow it (and invert it) for the sure background (`eroded` and `dilated`), keep only the photo's color inside each, and blur them outwards so they fill in under the uncertain band at the edge. The pictures must all be the same size, or it panics.

## Trace the edges of a mask

`contours` finds the outlines of the shapes in a mask, as closed loops of points, so a shape can be drawn as a path (stroked, filled, simplified or smoothed) and not only as pixels. It uses marching squares. To draw outlines, `outlined` does the whole job in one call: it traces each shape, drops the points the shape does not need, curves through the rest, strokes the result and gives a mask of the lines:

```rust
// Lines 3 pixels thick along the edge of every shape, within 1.5 pixels of it, with corners sharper than 50 degrees kept.
let lines = mask.outlined(128, 1.5, 50.0, 3.0);
lines.fill_image(&mut image, Color::black());
```

The lines are smooth curves with sharp corners kept, so they stay clean at any size, and the result is a mask the size of the mask it came from (`255` on a line). It is made in one go, so the points of the outlines are never copied out one by one; on a 600 x 1200 photo, the outline of a person's skin takes about 0.15 seconds. `traced` gives the shapes themselves as smooth areas, to stroke, fill or clip as you like:

```rust
for area in mask.traced(128, 1.5, 50.0) {
  let line = area.stroke(3.0).with_join(LineJoin::Round).to_area();
  fill(line, Color::black()).apply(&mut image);
}
```

A pixel is inside a shape when its value is at or above `level` (`1` to `255`; `0` counts as `1`). Where two neighbors are on either side of the level, the outline crosses between them in proportion to their values, so the edge of a soft mask is where the mask passes the level and not on the pixel grid. Each point is in the pixels of the mask, where the pixel at `x`, `y` has its middle at `x + 0.5`, `y + 0.5`. A shape that touches the edge of the mask is closed along the edge.

An outer outline runs counterclockwise as seen on the screen (the inside is on the left) and the outline of a hole runs clockwise, so filling the loops with the even-odd rule, or `polygon_difference` and `polygon_union`, keeps the holes. A mask of hard `0` and `255` values has its corners cut by half a pixel, because the points sit on the sides between pixels; simplifying the outline takes the steps out and keeps that cut small next to any real shape. See [Paths](../geometry/paths) for `simplify_points` and `smoothed_area`, and [Strokes](../geometry/strokes) for a stroke that tapers.

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
| `fill_image(image, fill)`                 | Paint a color, gradient or picture where the mask is on. |
| `opened(radius)` / `closed(radius)`       | Take away specks, or fill holes.                |
| `from_brightness_range(photo, dark, light, feather)` | A mask of the pixels between two brightnesses. |
| `brightness_percentile(photo, fraction, threshold)` | The brightness at a share of the way through the covered pixels. |
| `color_in(photo, stat, threshold)`        | The average, median or mode color of the part of a photo the mask covers. |
| `from_color_mix(photo, fg, bg, fallback, least)` | The share of a foreground color in each pixel. |
| `contours(level)`                         | The outlines of the shapes in the mask, as loops of points. |
| `traced(level, tolerance, corner)`        | The shapes in the mask as smooth areas. |
| `outlined(level, tolerance, corner, width)` | A mask of lines along the edges of the shapes. |
| `guided_filter(mask, photo, radius, eps)` | Make a mask follow the edges of a photo (color, CPU). |
| `guided(photo, radius, eps)`              | The same by brightness, as an effect on the CPU or GPU. |
