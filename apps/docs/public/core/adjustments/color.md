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

## API summary

| API                                     | Purpose                                            |
| --------------------------------------- | -------------------------------------------------- |
| `grayscale().apply(image)`              | Convert RGB to luminance grayscale.                |
| `invert().apply(image)`                 | Invert RGB channels.                               |
| `threshold(value).apply(image)`         | Convert pixels to black or white.                  |
| `posterize(levels).apply(image)`         | Reduce the number of RGB tonal levels.             |
| `reduce_opacity(value).apply(image)`    | Scale the alpha channel.                           |
| `auto_tone().apply(image)`              | Stretch per-channel tonal ranges.                  |
| `auto_color().apply(image)`              | Stretch levels and neutralize midtone color casts. |
| `gradient_map(gradient).apply(image)`   | Map luminance to gradient colors.                  |
