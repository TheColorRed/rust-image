---
title: Color
order: 1
outline: deep
---

# Color

`Color` stores an RGBA color using four 8-bit channels. Red, green, blue, and alpha values each range from `0` to `255`.

```rust
use abra::abra_core::Color;

let opaque_red = Color::from_rgb(255, 0, 0);
let translucent_red = Color::from_rgba(255, 0, 0, 128);
```

## Creating colors

### RGB and RGBA

Use `from_rgb` for an opaque color. Use `from_rgba` when the alpha channel matters:

```rust
let blue = Color::from_rgb(0, 0, 255);
let translucent_blue = Color::from_rgba(0, 0, 255, 160);
```

An alpha value of `0` is fully transparent and `255` is fully opaque.

### Hexadecimal

`from_hex` accepts a six-digit `0xRRGGBB` value and creates an opaque color:

```rust
let orange = Color::from_hex(0xFF7F00);
```

The hexadecimal input does not include alpha. Use `from_rgba` when an alpha channel is needed.

### HSL, HSV, and Lab

Create colors from HSL, HSV, or CIE Lab components:

```rust
let hsl_red = Color::from_hsl(0.0, 1.0, 0.5);
let hsv_red = Color::from_hsv(0.0, 1.0, 1.0);
let lab_red = Color::from_lab(53.2, 80.1, 67.2);
```

Hue is expressed in degrees. Saturation, value, and lightness use the `0.0` to `1.0` range. Lab uses `L` from `0` to `100` and `a`/`b` roughly from `-128` to `127` (D65 white point).

### Default and transparent colors

`Color::default()` returns opaque black. Use `transparent` for `(0, 0, 0, 0)`:

```rust
let black = Color::default();
let transparent = Color::transparent();
```

## Named colors

The color module provides constructors for common colors such as `red`, `green`, `blue`, `orange`, `purple`, `tan`, `white`, and `black`:

```rust
let accent = Color::royal_blue();
let background = Color::black();
let overlay = Color::white();
```

See [Named colors](./named-colors) for the complete list and channel values.

## Reading color channels

Read RGB or RGBA channels as tuples:

```rust
let color = Color::from_rgba(20, 80, 160, 200);

let (r, g, b) = color.rgb();
let (r, g, b, a) = color.rgba();
```

`rgb` and `rgba` return the original 8-bit channel values. `rgb` omits alpha; `rgba` includes it.

## Converting color spaces

Convert an RGB color to HSL, HSV, or Lab. The alpha-preserving variants return the alpha channel as a normalized value:

```rust
let color = Color::from_rgba(80, 140, 220, 192);

let (h, s, l) = color.hsl();
let (h, s, v) = color.hsv();
let (h, s, l, alpha) = color.hsla();
let (h, s, v, alpha) = color.hsva();
let (l, a, b) = color.lab();
```

To convert between two non-RGB spaces, go through `Color`, for example `Color::from_hsl(h, s, l).hsv()`.

`hsl` and `hsv` return three components. `hsla` and `hsva` return alpha as `f32` in the `0.0` to `1.0` range.

## Luminance and contrast

Use `luminance` to get the WCAG relative luminance, from `0.0` for black to `1.0` for white. Use `contrast_ratio` to compare two colors:

```rust
let foreground = Color::white();
let background = Color::black();

let brightness = foreground.luminance();
let contrast = foreground.contrast_ratio(background);
```

The contrast ratio follows WCAG and ranges from `1.0` for equal luminance to `21.0` for the strongest black-and-white contrast. `Color::black_white_contrast(color)` returns whichever of black or white is more readable on `color`.

For per-pixel brightness in image processing, use `luma(r, g, b, LumaStandard::Rec601)` (or `Rec709`), which weights the channels without linearizing them.

## Color statistics

`Color::from_pixels` reduces raw RGB or RGBA pixel data to a representative color using a `ColorStat`:

```rust
use abra::abra_core::{Channels, Color, ColorStat};

let pixels = [
  255, 0, 0, 255,
  0, 255, 0, 255,
  0, 0, 255, 255,
];

let average = Color::from_pixels(&pixels, Channels::RGBA, ColorStat::Average);
let median = Color::from_pixels(&pixels, Channels::RGBA, ColorStat::Median);
let mode = Color::from_pixels(&pixels, Channels::RGBA, ColorStat::Mode);
```

The `Channels` argument states the buffer layout. The alpha channel is not used to calculate the resulting color, and the returned color is opaque. An empty buffer returns `Color::transparent()`.

`Median` selects the middle value independently for red, green, and blue. `Mode` selects the most frequently occurring value independently for each channel.

## Color harmonies

`harmony` generates a color scheme from a source color. Every scheme includes the source color, and alpha is preserved:

```rust
use abra::abra_core::{Color, Harmony};

let triadic = Color::red().harmony(Harmony::Triadic); // red, green, blue
let shades = Color::royal_blue().harmony(Harmony::Shades(5));
```

| `Harmony`            | Colors                                                                     |
| -------------------- | -------------------------------------------------------------------------- |
| `Analogous`          | Five hues at -30, -15, 0, 15, and 30 degrees, source in the middle.         |
| `Complementary`      | The source and its opposite hue.                                            |
| `SplitComplementary` | Five colors: source, the hues beside its complement, and two darker tones.  |
| `Triadic`            | The source and the hues 120 and 240 degrees away.                           |
| `Square`             | The source and the hues 90, 180, and 270 degrees away.                      |
| `Compound`           | The source, a hue 30 degrees away, and the hue opposite that one.           |
| `Shades(n)`          | `n` progressively darker shades, starting at the source.                    |
| `Monochromatic(n)`   | `n` lightness variants of the source hue, source in the middle.             |

`Gradient::harmony(color, harmony)` builds an evenly spaced gradient from the same schemes.

## Using colors as fills

A `Color` can be passed directly anywhere Abra accepts a `Fill`:

```rust
use abra::abra_core::{Area, Color};
use abra::drawing::prelude::fill;

let area = Area::rect((0, 0), (320, 180));
let image = fill(area, Color::from_rgba(40, 120, 220, 220)).to_image();
```

See [Fill](./fill) for solid, gradient, and image fill workflows.

## API summary

| API                                 | Purpose                                      |
| ----------------------------------- | -------------------------------------------- |
| `Color::from_rgb(r, g, b)`          | Create an opaque RGB color.                  |
| `Color::from_rgba(r, g, b, a)`      | Create an RGBA color.                        |
| `Color::from_hex(0xRRGGBB)`         | Create an opaque color from hexadecimal RGB. |
| `Color::from_hsl(h, s, l)`          | Create a color from HSL values.              |
| `Color::from_hsv(h, s, v)`          | Create a color from HSV values.              |
| `Color::from_lab(l, a, b)`          | Create a color from CIE Lab values.          |
| `rgb`, `rgba`                       | Read channel values.                         |
| `hsl`, `hsla`, `hsv`, `hsva`, `lab` | Convert to color-space components.           |
| `luminance`                         | WCAG relative luminance, `0.0` to `1.0`.     |
| `contrast_ratio`                    | WCAG contrast from `1.0` to `21.0`.          |
| `Color::from_pixels(px, ch, stat)`  | Calculate a color from packed pixel data.    |
| `harmony(Harmony)`                  | Generate a color scheme.                     |
