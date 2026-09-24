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

### HSL and HSV

Create colors from HSL or HSV components:

```rust
let hsl_red = Color::from_hsl(0.0, 1.0, 0.5);
let hsv_red = Color::from_hsv(0.0, 1.0, 1.0);
```

Hue is expressed in degrees. Saturation, value, and lightness use the `0.0` to `1.0` range.

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
let channels = color.as_u8();
```

`rgb`, `rgba`, and `as_u8` return the original 8-bit channel values. `rgb` omits alpha; `rgba` and `as_u8` include it.

## Converting color spaces

Convert an RGB color to HSL or HSV. The alpha-preserving variants return the alpha channel as a normalized value:

```rust
let color = Color::from_rgba(80, 140, 220, 192);

let (h, s, l) = color.hsl();
let (h, s, v) = color.hsv();
let (h, s, l, alpha) = color.hsla();
let (h, s, v, alpha) = color.hsva();
```

`hsl` and `hsv` return three components. `hsla` and `hsva` return alpha as `f32` in the `0.0` to `1.0` range.

## Luminance and contrast

Use `luminance` to get a relative brightness value between `0.0` and `1.0`. Use `contrast_ratio` to compare two colors:

```rust
let foreground = Color::white();
let background = Color::black();

let brightness = foreground.luminance();
let contrast = foreground.contrast_ratio(background);
```

The contrast ratio ranges from `1.0` for equal luminance to `21.0` for the strongest black-and-white contrast.

## Color statistics

For raw RGB or RGBA pixel data, `average`, `median`, and `mode` calculate a representative RGB color:

```rust
let pixels = [
  255, 0, 0, 255,
  0, 255, 0, 255,
  0, 0, 255, 255,
];

let average = Color::average(&pixels);
let median = Color::median(&pixels);
let mode = Color::mode(&pixels);
```

The input is interpreted as packed RGB or RGBA samples. The alpha channel is not used to calculate the resulting color, and the returned color is opaque.

`median` selects the middle value independently for red, green, and blue. `mode` selects the most frequently occurring value independently for each channel.

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
| `rgb`, `rgba`, `as_u8`              | Read channel values.                         |
| `hsl`, `hsla`, `hsv`, `hsva`        | Convert to color-space components.           |
| `luminance`                         | Calculate brightness from `0.0` to `1.0`.    |
| `contrast_ratio`                    | Calculate contrast from `1.0` to `21.0`.     |
| `average`, `median`, `mode`         | Calculate a color from packed pixel data.    |
