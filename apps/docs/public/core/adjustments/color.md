---
title: Color operations
order: 3
outline: deep
---

# Color operations

The `color` adjustment module contains direct palette and channel operations. These functions mutate the supplied image; regional processing is documented in [Apply options](../apply-options).

## Grayscale

`grayscale` converts RGB channels using a luminance-weighted average while preserving alpha:

```rust
use abra::adjustments::prelude::color;

color::grayscale().apply(&mut image);
```

## Invert

`invert` replaces each RGB channel with its `255 - value` complement:

```rust
color::invert().apply(&mut image);
```

The alpha channel is not inverted.

## Threshold

`threshold` converts the image to black and white using the average RGB value of each pixel:

```rust
color::threshold(128).apply(&mut image);
```

Pixels with average brightness above the threshold become white; pixels at or below it become black. The threshold is clamped to `0..=255`.

## Posterize

`posterize` reduces RGB channels to a fixed number of levels:

```rust
color::posterize(6).apply(&mut image);
```

The level count is clamped to `2..=255`. Lower values create a more graphic, banded result.

## Quantize

`quantize` merges the colors of an image into a few. It groups the colors by how alike they look (in the Lab color space), works out the average color of each group, and paints every pixel with the average color of its group. Unlike `posterize`, which cuts each channel on its own and can shift hues, the colors come from the picture itself, so a photo of a red apple on a green table keeps a red and a green:

```rust
color::quantize(5).apply(&mut image);
```

The groups are found by k-means on a sample of the pixels, so the same image always gives the same colors. Tune the search with `with_iterations` (default 8) and `with_sample` (default 20000 pixels); every pixel is still painted.

`with_tolerance(distance)` merges colors that are alike. After the groups are found, any two whose average colors are closer than `distance` in the Lab color space (about 2 is the least a person can see, 10 is clearly different, 100 is black against white) are merged, nearest first, into the average of the two, each counted as many times as it has pixels. The number of colors given to `quantize` is then the most there will be. A face that was found as a light, a middle and a dark skin color becomes one skin color, while black, white and a gray stay apart. The default of `0` merges nothing.

`with_lightness(weight)` sets how much a difference in brightness counts, against a difference in color, when deciding how alike two colors are: both when found colors are merged and when each pixel takes the nearest color. The default of `1.0` is the Lab distance as it is. A lower weight such as `0.6` makes the lit and the shaded part of one surface look more alike, since they differ mostly in brightness, and makes a gray look nearer to another gray than to a tan of the same brightness. Without it a pale gray shadow can be nearer a pale tan than the white beside it, and takes the tan's color. Black and white stay apart while the weight is not far below `1.0`. It is not used to find the groups.

To paint with colors you choose, give a palette. Each pixel takes the nearest palette color, and this form runs on the GPU for up to 64 colors. A [`Palette`](../color/palette) can be given as it is, and there are ready made ones such as `Palette::comic()`:

```rust
color::quantize(0).with_palette([Color::from_rgb(232, 28, 92), Color::from_rgb(255, 214, 0)]).apply(&mut image);
```

`palette(&image)` returns the colors an image would be painted with, so they can be found once and reused:

```rust
let palette = color::quantize(5).palette(&image);
color::quantize(0).with_palette(palette).apply(&mut image);
```

To replace each color of a picture with a color of your choosing, give `with_sources` the colors that are there and `with_palette` what each should become. A pixel takes the palette color that goes with the source color nearest to it, so every color is swapped for its own new color:

```rust
let found = color::quantize(6).with_tolerance(25.0).palette(&image);
let bolder = found.iter().map(|color| make_bolder(color));
color::quantize(0).with_sources(found.clone()).with_palette(bolder).apply(&mut image);
```

The sources and the palette need the same number of colors; otherwise the sources are not used.

Finding the colors looks at the whole image, so it always runs on the CPU. With a given palette the result of each pixel depends only on that pixel.

## Opacity

Reduce the alpha channel by a factor from `0.0` to `1.0`:

```rust
color::reduce_opacity(0.65).apply(&mut image);
```

A factor of `0.0` makes pixels fully transparent; `1.0` leaves alpha unchanged. RGB channels are preserved.

## Automatic tone

`auto_tone` builds per-channel histogram levels and stretches the image while skipping fully transparent pixels:

```rust
color::auto_tone().apply(&mut image);
```

This is useful for correcting a narrow tonal range without manually calculating channel bounds.

## Automatic color

`auto_color` combines per-channel levels stretching with midtone color neutralization:

```rust
color::auto_color().apply(&mut image);
```

Restrict automatic color correction to an area or mask before applying it:

```rust
use abra::abra_core::Area;
use abra::options::prelude::ApplyOptions;

let area = Area::rect((100, 80), (320, 240));
color::auto_color()
  .with_options(ApplyOptions::new().with_area(area))
  .apply(&mut image);
```

Fully transparent pixels are ignored during analysis and their alpha values are preserved.

## Gradient maps

`gradient_map` converts each pixel to a luminance value and samples a `Gradient` at that normalized brightness:

```rust
use abra::abra_core::Gradient;

let gradient = Gradient::from_to(
  abra::abra_core::Color::black(),
  abra::abra_core::Color::orange(),
);
color::gradient_map(gradient).apply(&mut image);
```

Dark source pixels use the first gradient stop; bright source pixels use the last. Reverse the gradient before applying it when the tonal mapping should run in the opposite direction:

```rust
color::gradient_map(Gradient::rainbow().reverse()).apply(&mut image);
```

Gradient maps replace RGB channels with sampled gradient colors. See [Gradient](../color/gradient) for custom stops and direction behavior.

## Selective color

`selective_color` changes the colors in one range of a picture without touching the others, as the cyan, magenta, yellow and black inks of a print. There are nine ranges: `Reds`, `Yellows`, `Greens`, `Cyans`, `Blues`, `Magentas`, which are the colors, and `Whites`, `Neutrals` and `Blacks`, which are the light, middle and dark tones whatever their color.

```rust
use abra::adjustments::prelude::color::{ColorRange, SelectiveMethod, selective_color};

// Warm the middle tones and lighten the darkest parts, leaving every color range alone.
selective_color()
  .with_range(ColorRange::Neutrals, 0.0, 0.0, 0.3, -0.2)
  .with_range(ColorRange::Blacks, 0.0, 0.0, 0.0, -0.4)
  .apply(&mut image);
```

Each range is given a change of `[cyan, magenta, yellow, black]`: positive adds ink (darker, more of that color) and negative takes it away. Inks are worked out as in a print: the black ink is the darkest part of the pixel, and the cyan, magenta and yellow are what is left of each once the black is taken out, so a dark pixel has only a little of the three and changing its cyan changes only that little, while a black change lightens or darkens all of it. By default a change is a share of the ink a pixel already has (`SelectiveMethod::Relative`), so a pixel with none of that ink is not changed; `with_method(SelectiveMethod::Absolute)` adds a fixed amount instead. A pixel belongs to several ranges at once and each range's change is scaled by how much it belongs: the primary it is most of (by `max - mid`), the secondary it is least of (by `mid - min`), the whites by `min`, the blacks by `1 - max`, and the neutrals by how near both `max` and `min` are to half. The alpha is left alone, and the GPU version can differ from the CPU by one level.

## API summary

| API                                     | Purpose                                            |
| --------------------------------------- | -------------------------------------------------- |
| `grayscale().apply(image)`              | Convert RGB to luminance grayscale.                |
| `invert().apply(image)`                 | Invert RGB channels.                               |
| `threshold(value).apply(image)`         | Convert pixels to black or white.                  |
| `posterize(levels).apply(image)`         | Reduce the number of RGB tonal levels.             |
| `quantize(colors).apply(image)`         | Merge similar colors into their group's average.   |
| `reduce_opacity(value).apply(image)`    | Scale the alpha channel.                           |
| `auto_tone().apply(image)`              | Stretch per-channel tonal ranges.                  |
| `auto_color().apply(image)`              | Stretch levels and neutralize midtone color casts. |
| `gradient_map(gradient).apply(image)`   | Map luminance to gradient colors.                  |
| `selective_color().with_range(..)`      | Change the inks of one range of colors or tones.   |
