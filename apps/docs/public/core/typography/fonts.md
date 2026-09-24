---
title: Fonts
order: 1
outline: deep
---

# Fonts

Use `FontLoader` to target the exact font directory or glob you care about, and then load the matching file. In most cases this is faster and simpler than loading every system font and then filtering the list later.

## Discover fonts on the machine

If you already know the family or the directory, prefer a focused search over a full system scan.

```rust
use abra::typography::prelude::FontLoader;

let loader = FontLoader::find_fonts("assets/fonts/*.ttf");
let font = loader.load("Montserrat").unwrap_or_else(|| {
  panic!("Montserrat not found in the local font directory");
});
println!("Loaded font: {:?}", font);
```

If you already know the exact font file, prefer the direct path-based API instead of scanning for a family name:

```rust
use abra::typography::prelude::FontLoader;

let font = FontLoader::load_font("assets/fonts/Montserrat.ttf").unwrap();
println!("Loaded font: {:?}", font);
```

The loader is still useful when you want to search a known directory or glob for a family name without scanning the entire OS font library.

## Load a font

```rust
use std::fs;
use abra::typography::prelude::Font;

let font_bytes = fs::read("assets/fonts/MyFont.ttf")?;
let font = Font::from_bytes(font_bytes)?;

let same_font = Font::from_path("assets/fonts/MyFont.ttf")?;
```

`Font::from_path` reads a `.ttf` or `.otf` file from disk. `Font::from_bytes` accepts the raw font data directly, which is convenient when embedding assets or streaming files from a package. For most application code, `from_path` is the simplest option when the target font is already known.

## Variable fonts and weight

Variable fonts can expose a `wght` axis. When you call `Text::with_weight`, the text renderer selects that weight instance during rasterization.

```rust
use abra::typography::prelude::Font;

let font = Font::from_path("assets/fonts/Inter-Variable.ttf")?;
let text = font
  .text("Hello")
  .with_size(64.0)
  .with_weight(700);
```

A weight of `400` is regular, `700` is bold, and other values in the `100..=900` range are supported by the underlying font if the file includes them.

## FontLoader API summary

| API                                | Purpose                                                   |
| ---------------------------------- | --------------------------------------------------------- |
| `FontLoader::find_system_fonts()`  | Collect installed fonts from the OS font directories.     |
| `FontLoader::find_fonts(patterns)` | Discover fonts from glob patterns or explicit file paths. |
| `add_fonts(patterns)`              | Append more font locations to an existing loader.         |
| `fonts()`                          | Read the discovered font file paths.                      |
| `load(name)`                       | Find a matching font and return a loaded `Font` instance. |
| `load_font(path)`                  | Load a font directly from an exact file path.             |
| `at(index)`                        | Access a font by index.                                   |

## Relevant types

- `Font` loads the font face and prepares it for glyph metrics and rasterization.
- `FontLoader` is the discovery and selection helper for locating fonts in a project or system installation.

Use [Text](./text) to turn a loaded font into an image-backed text layer.
