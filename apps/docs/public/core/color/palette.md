---
title: Palette
order: 6
outline: deep
---

# Palette

A `Palette` is a list of colors to choose from. It answers one question: which of these colors is nearest to this one? Colors are compared by how different they look, which is the distance between their Lab values, and not by the distance between their red, green and blue values. A brown is then matched to the orange that looks like it, not to a gray that happens to have similar channel numbers.

```rust
use abra::abra_core::{Color, Palette};

let inks = Palette::silk_screen();
let ink = inks.nearest(Color::from_rgb(180, 90, 40)); // the silk screen color that looks most like this brown
let place = inks.nearest_index(Color::from_rgb(180, 90, 40)); // and where it is in the palette
```

`nearest` and `nearest_index` give `None` for a palette with no colors. When two colors are equally near, the first wins. The color you get back is the palette's own, alpha included, and the alpha of the color you look up is ignored.

## Making a palette

```rust
let mine = Palette::new([Color::red(), Color::from_rgb(255, 214, 0), Color::black()]);
let grays = Palette::grayscale(16); // 16 grays from black to white
```

`colors()` returns the colors in order, and a palette can be walked as colors, so it can be given to anything that takes colors, such as the palette of [quantize](../adjustments/color#quantize).

## Tones

`tones(steps)` turns each color into `steps` colors of its own hue, from dark to light, as the shades and tints of one ink. A palette of 8 inks with 8 steps is a palette of 64 colors:

```rust
let inks = Palette::new([Color::from_rgb(20, 40, 150), Color::from_rgb(230, 30, 40)]);
let blues_and_reds = inks.tones(8); // 16 colors: 8 blues, then 8 reds
```

The steps go from near black to near white, and the brightness goes up evenly. The step nearest to a color's own brightness is the color itself, exactly, so the ink you gave is always in the result. The steps either side of it keep its hue and lose color the further they are from it, down to a little under half at the two ends, where a screen cannot show much. A bright color such as a yellow therefore has more shades than tints, and a dark one such as a navy has more tints than shades. A `steps` of 1 gives the color itself.

## Ready made palettes

| Palette                   | Colors | What it is                                                                          |
| ------------------------- | ------ | ----------------------------------------------------------------------------------- |
| `Palette::comic()`        | 64     | The inks of a comic book: red, magenta, orange, yellow, lime, green, teal, cyan, blue and peach, each in 5 tones, and 14 grays from black to white. |
| `Palette::silk_screen()`  | 64     | Navy, hot pink, yellow, turquoise, red-orange, violet, green and gray, each in 8 tones. |
| `Palette::sepia()`        | 64     | Browns from dark to light, as an old photograph is printed.                         |
| `Palette::neon()`         | 6      | Hot pink, cyan, green, yellow and purple on near black.                             |
| `Palette::pastel()`       | 7      | Pink, peach, yellow, mint, sky blue and lavender, and white.                        |
| `Palette::cga()`          | 4      | Black, cyan, magenta and white, the high intensity palette of early PC graphics.    |
| `Palette::game_boy()`     | 4      | The four greens of the original Game Boy screen.                                    |

## Painting an image with a palette

`quantize` with a palette paints every pixel with the nearest color of the palette, and runs on the GPU for a palette of up to 64 colors:

```rust
use abra::adjustments::prelude::color;

color::quantize(0).with_palette(Palette::comic()).apply(&mut image);
```

To replace the colors a photo has, one for one, find them first and give them as `with_sources`. See [Quantize](../adjustments/color#quantize).
