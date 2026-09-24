---
title: Typography Overview
order: 0
outline: deep
---

# Typography Overview

Abra's typography APIs load font files and rasterize text into images. They are designed for compositing labels, titles, callouts, and typographic overlays into a canvas.

## Typography guides

| Guide            | Covers                                                                            |
| ---------------- | --------------------------------------------------------------------------------- |
| [Fonts](./fonts) | Discover system or project fonts and load them from file paths or bytes.          |
| [Text](./text)   | Build styled text objects with sizing, colors, weights, spacing, and line-height. |

## Typical workflow

When you already know the font family or a specific folder of fonts, prefer a targeted `FontLoader` lookup instead of scanning every installed system font.

```rust
use abra::canvas::prelude::*;
use abra::prelude::*;
use abra::typography::prelude::{Font, FontLoader};

let loader = FontLoader::find_fonts("assets/fonts/*.ttf");
let font = loader.load("Montserrat").unwrap();

let text = font
  .text("Abra")
  .with_size(220.0)
  .with_fill(Color::ruby())
  .with_weight(800)
  .with_letter_spacing(28.0);

let canvas = Canvas::new_blank("Title", 1400, 700);
canvas.add_layer_from_image("Title", text, None);
canvas.save("out/typography.png", None);
```

When the file path is already known, prefer the direct call `FontLoader::load_font("assets/fonts/Montserrat.ttf")` instead of doing a search. Use `FontLoader::find_system_fonts()` only when you intentionally want a broad system-wide search. For known font directories or known families, `find_fonts()` plus `load()` is the faster, more focused workflow. Use [Canvas](../canvas/) to composite text over other layers and [Drawing](../drawing/) for additional rasterized effects.
