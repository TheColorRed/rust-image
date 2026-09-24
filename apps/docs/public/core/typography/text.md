---
title: Text
order: 2
outline: deep
---

# Text

A `Text` object is built from a loaded `Font` and can be styled before being converted into an image. It supports direct conversion into `Image` or `Arc<Image>`, which makes it easy to add to a canvas as a layer.

## Build styled text

```rust
use abra::prelude::*;
use abra::typography::prelude::Font;

let font = Font::from_path("assets/fonts/Montserrat.ttf")?;

let title = font
  .text("Abra\nKadabra")
  .with_size(220.0)
  .with_fill(Color::ruby())
  .with_weight(800)
  .with_alignment(TextAlign::Justify)
  .with_letter_spacing(24.0)
  .with_line_height(240.0);
```

## Add text to a canvas

```rust
use abra::canvas::prelude::*;
use abra::prelude::*;
use abra::typography::prelude::Font;

let font = Font::from_path("assets/fonts/Montserrat.ttf")?;
let title = font
  .text("Abra")
  .with_size(180.0)
  .with_fill(Color::white())
  .with_weight(700);

let canvas = Canvas::new_blank("Title", 1200, 800);
canvas.add_layer_from_image("Title", title, None);
```

## Multi-line output

`\n` is a normal line break within one paragraph. Chain `.text(...)` to append another paragraph and apply the configured paragraph spacing:

```rust
let text = font
  .text("Hello\nWorld")
  .text("A second paragraph")
  .with_size(48.0)
  .with_fill(Color::black())
  .with_line_height(56.0)
  .with_paragraph_spacing(24.0);
```

When `with_line_height` is unset, Abra uses a tight default line advance of 0.9x the font size. Chained paragraphs receive an additional default gap of 0.6x the font size, so a paragraph boundary advances by 1.5x the font size in total. Use `with_line_height` and `with_paragraph_spacing` to choose a more open body-text treatment.

The renderer uses the actual glyph bounds to size the output image, so the image naturally matches the rendered text rather than a box-based placeholder.

## Alignment and justification

`with_alignment` controls how every line is positioned relative to the text block's natural longest line. Use `TextAlign::Left`, `Center`, or `Right` for the corresponding alignment. `TextAlign::Justify` expands the spaces between words so shorter lines reach the width of the longest rendered line.

```rust
let title = font
  .text("Wide title\nNarrow title")
  .with_size(72.0)
  .with_alignment(TextAlign::Justify);
```

`with_justification(TextJustify::InterWord)` enables the same word-spacing behavior without changing the selected alignment. Without `with_width`, the natural longest rendered line is the layout target. When a width is supplied, it becomes the explicit alignment and justification target; the renderer still sizes the output image from the actual glyph bounds.

## Word wrapping

Wrapping is opt-in. Pair `with_word_wrap` with `with_width` to split text before alignment and justification are applied. The width is the maximum line width used by the wrapping algorithm.

```rust
use abra::typography::prelude::WordWrap;

let paragraph = font
  .text("A longer title that should fit inside a narrow text block")
  .with_size(48.0)
  .with_width(520.0)
  .with_word_wrap(WordWrap::BreakWord)
  .with_alignment(TextAlign::Justify);
```

The available wrapping modes are:

- `WordWrap::None`: Preserve each explicit newline; this is the default.
- `WordWrap::Normal`: Wrap only at whitespace. A single word that is wider than `with_width` remains on its own line.
- `WordWrap::BreakWord`: Wrap at whitespace, then split a word only when it cannot fit on its own line.
- `WordWrap::Anywhere`: Break at any character as needed to fit the configured width.

When wrapping is enabled, the configured width controls where the source text is split and the alignment or justification target for the resulting lines.

## Automatic text size

Pass `TextSize::Auto` to `with_size` with an explicit `with_width`, `with_height`, or both to size text into a constrained area.

```rust
use abra::typography::prelude::TextSize;

let title = font
  .text("A title that fills its layout width")
  .with_size(TextSize::Auto)
  .with_width(960.0)
  .with_height(240.0);
```

Automatic sizing is resolved before the text becomes an image, so it uses `with_width` and `with_height` rather than a future canvas or layer size. When both are set, the smaller fitting size wins. Without either constraint, `TextSize::Auto` falls back to the default 32px size.

`with_height` also constrains fixed-size text. It derives a maximum line count from the available vertical space and applies `TextOverflow::Clip` or `TextOverflow::Ellipsis`, just like `with_max_lines`.

`TextSize::percent` uses the smaller configured layout axis as its reference. `TextSize::points` converts typographic points to pixels using 96 DPI by default; call `with_dpi` when rendering for a different output resolution.

```rust
let label = font
  .text("Print label")
  .with_size(TextSize::points(18.0))
  .with_dpi(300.0);

let responsive_label = font
  .text("Responsive label")
  .with_size(TextSize::percent(8.0))
  .with_width(640.0);
```

## Canvas DPI inheritance

`Canvas::add_layer_from_image` automatically rasterizes `Text` with the canvas document DPI and stores that resolution on the resulting layer image. This is the preferred path for point-sized text. A root canvas propagates its resolution to every attached child canvas and layer image, including descendants added before or after `set_resolution` is called. Retained text layers are re-rasterized at the inherited DPI when that propagation occurs. Calling `set_resolution` directly on a child creates an override, so later root changes do not replace that child's DPI.

```rust
use abra::abra_core::Resolution;
use abra::canvas::prelude::Canvas;
use abra::typography::prelude::TextSize;

let root = Canvas::new_blank("Composition", 1200, 800);
root.set_resolution(Resolution::PRINT);

let title_canvas = root.add_canvas(
  Canvas::new_blank("Title", 1200, 800),
  None,
);
title_canvas.add_layer_from_image(
  "Title",
  font.text("Print title").with_size(TextSize::points(18)),
  None,
);
```

When given an `Image`, `Canvas::add_layer_from_image` retains that image's existing resolution metadata. Use `Text::with_dpi` only when rendering text independently from a canvas.

## Line clamping and overflow

Use `with_max_lines` to limit wrapped output to a fixed number of lines, like CSS `line-clamp`. Pair it with `TextOverflow::Ellipsis` when truncated content should end with an ellipsis; `TextOverflow::Clip` is the default.

```rust
use abra::typography::prelude::{TextOverflow, WordWrap};

let summary = font
  .text("A long description that should occupy no more than two rendered lines.")
  .with_size(32.0)
  .with_width(480.0)
  .with_word_wrap(WordWrap::Normal)
  .with_max_lines(2)
  .with_overflow(TextOverflow::Ellipsis);
```

## Font styles and features

Variable font axes can be set directly with `with_variation`; `with_style` is a convenient shorthand for the common `ital` and `slnt` axes. Static fonts remain usable and simply ignore axes they do not provide.

```rust
use abra::typography::prelude::{TextDecoration, TextStyle};

let label = font
  .text("Office 2026")
  .with_size(48.0)
  .with_style(TextStyle::Italic)
  .with_variation("wdth", 90.0)
  .with_open_type_feature("liga=0")
  .with_open_type_feature("tnum=1")
  .with_decoration(TextDecoration::Underline);
```

Abra shapes every line before measuring or drawing it. This applies standard kerning, ligatures, contextual substitutions, and positioning for the font, including complex scripts. Use OpenType feature strings such as `"kern=0"`, `"liga=0"`, or `"tnum=1"` to override the font's defaults.

## Fill types

`with_fill` accepts any value that can be converted into Abra's `Fill` type. This includes:

- a solid `Color`
- a `Gradient` for multi-stop color transitions
- an `Image` for texture or pattern-based text fills

```rust
use abra::abra_core::{Color, Gradient};
use abra::prelude::*;
use abra::typography::prelude::Font;

let font = Font::from_path("assets/fonts/Montserrat.ttf")?;
let gradient = Gradient::from_to(Color::ruby(), Color::gold());

let title = font
  .text("Abra")
  .with_size(180.0)
  .with_fill(gradient)
  .with_weight(700);
```

This is the same fill concept used elsewhere in Abra: a text layer can be painted with a single color, a linear ramp, or an image source, depending on the effect you want.

## Text builder API summary

| API                             | Purpose                                                                   |
| ------------------------------- | ------------------------------------------------------------------------- |
| `font.text("Hello")`            | Create a text builder with its first paragraph.                           |
| `text("More")`                  | Append another paragraph to an existing text builder.                     |
| `with_size(px)`                 | Set the text size in pixels.                                              |
| `with_fill(fill)`               | Set a solid, gradient, or image fill.                                     |
| `with_weight(value)`            | Select a weight for variable fonts.                                       |
| `with_style(value)`             | Select `TextStyle::Normal`, `Italic`, or `Oblique`.                       |
| `with_variation(axis, value)`   | Set a variable-font axis such as `wdth` or `opsz`.                        |
| `with_open_type_feature(value)` | Override an OpenType feature, for example `liga=0`.                       |
| `with_decoration(value)`        | Add `TextDecoration::Underline` or `Strikethrough`.                       |
| `with_size(TextSize::Auto)`     | Fit text to the available `with_width` and/or `with_height` constraints.  |
| `with_size(TextSize::percent)`  | Set text size as a percentage of the constrained layout area.             |
| `with_size(TextSize::points)`   | Set text size in typographic points.                                      |
| `with_width(value)`             | Set the layout width for wrapping, alignment, and justification.          |
| `with_height(value)`            | Set the layout height for automatic sizing and overflow.                  |
| `with_dpi(value)`               | Set the DPI used for point-to-pixel conversion.                           |
| `with_word_wrap(value)`         | Select `WordWrap::None`, `Normal`, `BreakWord`, or `Anywhere`.            |
| `with_max_lines(value)`         | Limit rendered output to a fixed number of lines.                         |
| `with_overflow(value)`          | Choose `TextOverflow::Clip` or `Ellipsis` for clamped text.               |
| `with_alignment(value)`         | Align each line using `TextAlign::Left`, `Center`, `Right`, or `Justify`. |
| `with_justification(value)`     | Enable `TextJustify::InterWord` word-spacing justification.               |
| `with_line_height(value)`       | Override the line spacing between rows.                                   |
| `with_paragraph_spacing(value)` | Add spacing between chained paragraphs.                                   |
| `with_letter_spacing(value)`    | Add spacing between individual characters.                                |
| `with_word_spacing(value)`      | Add spacing between words.                                                |

## Notes

- `with_weight` is most useful with variable fonts, but it still accepts a value for static fonts and will apply the nearest available face if needed.
- Numeric text-layout APIs accept all primitive integer and floating-point values, so values such as `32`, `64.0`, and `220usize` are valid inputs.
- `Text` converts directly into `Image` and `Arc<Image>`, which fits naturally into the rest of Abra's layer API.

See [Fonts](./fonts) for loading the font face, and [Canvas](../canvas/) for layered composition.
