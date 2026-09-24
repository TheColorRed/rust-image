---
title: Gradient
order: 4
outline: deep
---

# Gradient

`Gradient` describes a sequence of colors and interpolates between them. A gradient is made from color stops, where each stop has a color and a normalized position from `0.0` to `1.0`.

Use a gradient anywhere Abra accepts a `Fill`, including `fill`.

## Quick start

Create a two-color gradient with `from_to`, set its direction with a line, and fill an area:

```rust
use abra::abra_core::{Area, Color, Gradient};
use abra::drawing::prelude::{fill, Path};

let gradient = Gradient::from_to(Color::black(), Color::white())
  .with_direction(Path::line((0, 0), (400, 0)));

let area = Area::rect((0, 0), (400, 300));
let image = fill(&area, &gradient).to_image();
```

The first point of the path is the start of the gradient and the last point is the end. In this example, the gradient runs horizontally from left to right.

## Creating gradients

### Two colors

`from_to` creates stops at `0.0` and `1.0`:

```rust
let gradient = Gradient::from_to(Color::red(), Color::blue());
```

For a fade to a neutral color, use `to_black` or `to_white`:

```rust
let shade = Gradient::to_black(Color::orange());
let tint = Gradient::to_white(Color::orange());
```

### Evenly spaced colors

`evenly` distributes the supplied colors across the full gradient. The first color is at `0.0` and the last is at `1.0`:

```rust
let gradient = Gradient::evenly(vec![
  Color::red(),
  Color::yellow(),
  Color::green(),
  Color::blue(),
]);
```

Abra also provides two ready-made palettes:

```rust
let rainbow = Gradient::rainbow();
let hue = Gradient::hue();
```

### Custom color stops

Use `ColorStop::new` when a color should remain longer or transition at a specific point. Keep stops in ascending time order.

```rust
use abra::abra_core::{Color, ColorStop, Gradient};

let gradient = Gradient::new(vec![
  ColorStop::new(Color::black(), 0.0),
  ColorStop::new(Color::red(), 0.25),
  ColorStop::new(Color::yellow(), 0.75),
  ColorStop::new(Color::white(), 1.0),
]);
```

## Direction

The direction is optional. Set it with `with_direction`, which takes any value convertible to a `Path`:

```rust
use abra::drawing::prelude::Path;

let vertical = Gradient::from_to(Color::purple(), Color::blue())
  .with_direction(Path::line((0, 0), (0, 300)));

let diagonal = Gradient::from_to(Color::yellow(), Color::red())
  .with_direction(Path::line((0, 0), (400, 300)));
```

When `fill` is given a gradient without an explicit direction, it uses the filled area's bounding box to create a horizontal fallback direction.

## Reading and transforming gradients

Sample a gradient at a normalized position with `get_color`:

```rust
let gradient = Gradient::from_to(Color::black(), Color::white());
let middle = gradient.get_color(0.5);
```

`get_color` returns an RGBA tuple. `get_color_type` returns the same value as a `Color`. Reverse the color progression with `reverse`:

```rust
let reversed = gradient.reverse();
```

## Applying a gradient to an image

The following example loads an image, creates a vertical multi-stop gradient, renders it over an area matching the image, and adds the result as a semi-transparent layer:

```rust
use abra::abra_core::ColorStop;
use abra::canvas::prelude::*;
use abra::drawing::prelude::*;
use abra::prelude::*;

const OUT_FILE: &str = "out/gradient.png";
const BACKGROUND_IMAGE: &str = "image/example.jpg";

pub fn main() {
  let canvas = Canvas::new("Gradient");
  let background = canvas.add_layer_from_path("background", BACKGROUND_IMAGE, None);

  let gradient = Gradient::new(vec![
    ColorStop::new(Color::tan(), 0.0),
    ColorStop::new(Color::purple(), 0.5),
    ColorStop::new(Color::blue(), 1.0),
  ])
  .with_direction(Path::line((0, 0), (0, background.dimensions().1)));

  let area = Area::new_from_image(&background.as_image());
  let gradient_image = fill(&area, &gradient).to_image();

  let options = NewLayerOptions::new().with_opacity(0.6);
  canvas.add_layer_from_image("gradient", gradient_image, options);
  canvas.save(OUT_FILE, None);
}
```

`with_opacity` clamps the layer opacity to the range `0.0` to `1.0`. A value of `0.0` makes the gradient layer transparent, while `1.0` makes it fully opaque.

## API summary

| API                           | Purpose                                             |
| ----------------------------- | --------------------------------------------------- |
| `Gradient::new(stops)`        | Create a gradient from explicit `ColorStop` values. |
| `Gradient::from_to(from, to)` | Create a two-color gradient.                        |
| `Gradient::to_black(from)`    | Fade a color to black.                              |
| `Gradient::to_white(from)`    | Fade a color to white.                              |
| `Gradient::evenly(colors)`    | Distribute colors evenly between `0.0` and `1.0`.   |
| `Gradient::rainbow()`         | Create a red-to-violet rainbow gradient.            |
| `Gradient::hue()`             | Create a hue-based gradient.                        |
| `.with_direction(path)`       | Set the start and end points of the gradient.       |
| `.get_color(time)`            | Sample an RGBA tuple at a normalized position.      |
| `.get_color_type(time)`       | Sample a `Color` at a normalized position.          |
| `.reverse()`                  | Return a gradient with its progression reversed.    |
