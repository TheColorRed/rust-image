---
title: Levels and color controls
order: 2
outline: deep
---

# Levels and color controls

The `levels` module contains tonal and color controls that operate on an image in place. Builders apply to the whole image by default; use [Apply options](../apply-options) for regional processing.

```rust
use abra::adjustments::prelude::levels;
use abra::prelude::*;

let mut image = Image::read("assets/photo.jpg")?;
levels::contrast(25).apply(&mut image);
image.write("out/contrast.png", None)?;
```

## Brightness

`brightness` accepts an integer amount. Zero is unchanged; positive values brighten and negative values darken:

```rust
levels::brightness(40).apply(&mut image);
levels::brightness(-20).apply(&mut image);
```

## Contrast

`contrast` uses a range of `-100..=100`, with zero leaving the image unchanged:

```rust
levels::contrast(35).apply(&mut image);
levels::contrast(-15).apply(&mut image);
```

The implementation clamps the amount to this range and adjusts RGB channels around the midpoint value `128`.

## Highlights and shadows

`highlights_shadows(highlights, shadows)` uses `-100..=100` for the basic sliders.
The native builder does not clamp supplied settings to the UI ranges.
Positive highlights lower bright tones; positive shadows lift dark tones. Negative
amounts reverse these directions, and `(0, 0)` leaves the pixels unchanged.

```rust
levels::highlights_shadows(40, 25)
  .with_shadow_tone(0.5)
  .with_shadow_radius(30)
  .with_highlight_tone(0.5)
  .with_highlight_radius(30)
  .apply(&mut image);
```

The smooth luminance curve avoids the tonal reversal caused by multiplying bright
pixels toward black. Separate Gaussian neighborhoods determine shadow and
highlight strength; the effect reaches a pixel only when both the pixel and its
neighborhood are in the tonal range. A common gain preserves linear RGB proportions,
and brightening stops at the gamut boundary. Alpha is preserved. Clipped whites
contain no detail to recover.

| Builder setting                               | Range                              | Default |
| --------------------------------------------- | ---------------------------------- | ------- |
| `with_shadow_tone`, `with_highlight_tone`     | 0–1 fraction                       | 0.5     |
| `with_shadow_radius`, `with_highlight_radius` | Nonnegative pixels, rounded to u32 | 30      |
| `with_color`                                  | −100–100 saturation correction     | 0       |
| `with_midtone`                                | −100–100 midtone contrast          | 0       |
| `with_black_clip`, `with_white_clip`          | Fraction of visible pixels         | 0       |

Tone and clipping use normalized fractions: `0.5` is 50%, and `0.0001` is 0.01%.
The ranges above describe the basic controls; they are not enforced by the builder.
Mask weights and output pixels are bounded as part of image processing.
`with_tone` sets both tonal widths; `with_radius` sets both radii. Radii are in the
processed image's pixels, so scale them with the image dimensions when previewing
at a smaller size. Tonal width zero disables that tonal adjustment. Zero amounts
with the default color, midtone and clipping settings leave the image unchanged.

Color correction reuses the saturation adjustment. Nonzero clipping uses an
8-bit luminance histogram to stretch the dark and bright percentiles, excluding
fully transparent pixels. Clipping runs on the CPU; the other settings support
the GPU. Flat histograms are left unstretched, and brightening remains gamut-limited.
These controls are inspired by Photoshop's Shadow/Highlight dialog; their math and
defaults are not intended to reproduce Photoshop pixel for pixel.

TypeScript builders use camelCase names:

```ts
import { highlightsShadows } from '@abra/core/adjustments';

highlightsShadows(40, 35)
  .withShadowTone(0.5)
  .withShadowRadius(30)
  .withHighlightTone(0.5)
  .withHighlightRadius(30)
  .withColor(20)
  .withMidtone(0)
  .withBlackClip(0.0001)
  .withWhiteClip(0.0001)
  .apply(image);
```

## Exposure

`exposure` takes the exposure in stops. Offset and gamma correction are optional:

```rust
levels::exposure(1.0).apply(&mut image);
levels::exposure(1.0).with_offset(0.05).with_gamma(1.2).apply(&mut image);
```

Offset defaults to `0.0` and gamma correction to `1.0`. The implementation clamps exposure to `-20..=20`, offset to `-0.5..=0.5`, and gamma correction to `0.01..=9.99`. Alpha is preserved.

Pass the desired stop value directly to `exposure`:

```rust
levels::exposure(1.0).apply(&mut image);
levels::exposure(-1.0).apply(&mut image);
levels::exposure(2.0).apply(&mut image);
levels::exposure(-2.0).apply(&mut image);
```

## Saturation

`saturation` accepts an amount from `-100..=100`. Zero is unchanged, `-100` approaches grayscale, and positive values increase color intensity:

```rust
levels::saturation(30).apply(&mut image);
levels::saturation(-100).apply(&mut image);
```

## Vibrance

`vibrance` adjusts less-saturated colors selectively. It can also apply an even saturation adjustment, which defaults to `0`:

```rust
levels::vibrance(35).apply(&mut image);
levels::vibrance(35).with_saturation(10).apply(&mut image);
```

Both vibrance and saturation values are clamped to `-100..=100`. The alpha channel remains unchanged.

## Hue

The intended `hue` API takes an amount in degrees, where zero means no change and values are conceptually in `-180..=180`:

```rust
levels::hue(30).apply(&mut image);
```

The current implementation is marked TODO and does not modify the image. Treat this function as a placeholder until hue rotation is implemented.

## Photo filters

Apply a custom filter color. The density, from `0.0` to `1.0`, defaults to `0.25`:

```rust
use abra::abra_core::Color;
use abra::adjustments::levels::PhotoFilter;

levels::photo_filter(PhotoFilter::Color(Color::from_rgb(255, 140, 40))).with_density(0.35).apply(&mut image);
```

Use `PhotoFilter::Preset` with a `FilterType` for predefined warming, cooling, spectral, and stylized filters:

```rust
use abra::adjustments::{FilterType, levels::PhotoFilter};

levels::photo_filter(PhotoFilter::Preset(FilterType::WarmingLight)).with_density(0.5).apply(&mut image);
```

Available presets include:

- `WarmingDark`, `WarmingLight`
- `CoolingDark`, `CoolingLight`
- `Red`, `Orange`, `Yellow`, `Green`, `Cyan`, `Blue`
- `Violet`, `Magenta`, `Sepia`
- `DeepRed`, `DeepBlue`, `DeepEmerald`, `DeepYellow`
- `Underwater`

## API summary

| API                                                    | Purpose                                              |
| ------------------------------------------------------ | ---------------------------------------------------- |
| `brightness(amount).apply(image)`                      | Increase or decrease brightness.                     |
| `contrast(amount).apply(image)`                        | Adjust contrast around midpoint gray.                |
| `highlights_shadows(highlights, shadows).apply(image)` | Adjust bright and dark tones while preserving color. |
| `exposure(stops).with_offset(..).with_gamma(..)`       | Adjust exposure, offset, and gamma.                  |
| `saturation(amount).apply(image)`                      | Increase or decrease saturation.                     |
| `vibrance(amount).with_saturation(..)`                 | Enhance color selectively and adjust saturation.     |
| `hue(amount).apply(image)`                             | Reserved for hue rotation; currently a no-op.        |
| `photo_filter(filter).with_density(..)`                | Apply a custom color or predefined filter.           |
