---
title: Fill
order: 3
---

# Fill

`Fill` describes what Abra should paint inside a closed `Area`. It supports three fill styles:

- `Color` for a solid color.
- `Gradient` for interpolated colors.
- `Image` for using another image as the source.

The drawing API accepts these values directly through `Into<Fill>`, so most code can pass a `Color`, `Gradient`, or `Image` to `fill` without constructing the enum manually.

## Filling an area

`fill` rasterizes an area and returns a new `Image`. The simplest example fills a rectangle with a solid color:

```rust
use abra::abra_core::{Area, Color};
use abra::drawing::prelude::fill;

let area = Area::rect((0, 0), (320, 180));
let image = fill(&area, Color::from_rgba(30, 120, 220, 255)).to_image();

image.write("out/blue-rectangle.png", None)?;
```

The output image is sized to the area's bounds. Coordinates in the area are preserved while the returned image is translated to image-local coordinates.

## Fill styles

### Solid color

Pass a `Color` directly to `fill` or use `Fill::Solid` when you need to store the fill style:

```rust
use abra::abra_core::{Color, Fill};

let solid = Fill::Solid(Color::orange().into());
```

The color's alpha channel is respected during source-over compositing. For example, an alpha value of `128` produces a semi-transparent fill.

### Gradient

Gradients can be passed directly as fills. Set a direction when the gradient should follow a specific axis or line:

```rust
use abra::abra_core::{Area, Color, Gradient};
use abra::drawing::prelude::{fill, Path};

let gradient = Gradient::from_to(Color::purple(), Color::blue())
  .with_direction(Path::line((0, 0), (0, 240)));
let area = Area::rect((0, 0), (320, 240));
let image = fill(&area, &gradient).to_image();
```

If a gradient has no explicit direction, `fill` uses a horizontal path across the area's bounding box. See the [Gradient](./gradient) guide for color stops, presets, and more direction examples.

### Image

An `Image` can also be used as a fill source:

```rust
use abra::abra_core::{Area, Image};
use abra::drawing::prelude::fill;

let texture = Image::from_path("assets/texture.png", None).unwrap();
let area = Area::rect((0, 0), (640, 480));
let image = fill(&area, &texture).to_image();
```

The image is sampled through the fill shader while the area controls which pixels are written. This is useful for applying a texture or image content to a custom shape.

## Shapes and feathering

`Area` provides helpers for common closed shapes. Use `circle`, `ellipse`, or `from_points` when a rectangle is not enough:

```rust
use abra::abra_core::{Area, Color};
use abra::drawing::prelude::fill;

let circle = Area::circle((160, 120), 80.0);
let image = fill(circle, Color::from_rgba(220, 60, 80, 255)).to_image();
```

Feather an area's edge with `with_feather`. The value is a radius in pixels, and the edge coverage fades instead of ending abruptly:

```rust
let soft_circle = Area::circle((160, 120), 80.0).with_feather(12);
let image = fill(soft_circle, Color::from_rgba(220, 60, 80, 255)).to_image();
```

## Drawing into an existing image

Use `apply` when the result should be composited into an existing `Image`. It creates the filled image and draws it using source-over compositing:

```rust
use abra::abra_core::{Area, Color, Image};
use abra::drawing::prelude::fill;

let mut canvas = Image::new(800, 600);
let area = Area::circle((100, 100), 80.0);

fill(area, Color::from_rgba(255, 180, 40, 220)).with_position((240, 160)).apply(&mut canvas);

canvas.save("out/composited.png", None);
```

Without `with_position`, `apply` draws the area where it is: the top-left of its bounds lands at the same point in the image. `with_position` moves the whole filled image; it does not change the geometry used to rasterize the area.

## Getting an image or drawing into one

| API                                       | Use when                                                   |
| ----------------------------------------- | ---------------------------------------------------------- |
| `fill(area, fill).to_image()`             | You need a new image containing the filled area.           |
| `fill(area, fill).apply(image)`           | You need to draw the filled result into an existing image. |
| `Fill::Solid(...)`                        | You need to store or pass a solid fill as a `Fill`.        |
| `Fill::Gradient(...)`                     | You need to store or pass a gradient as a `Fill`.          |
| `Fill::Image(...)`                        | You need to store or pass an image source as a `Fill`.     |

## Fill pipeline

For each fill operation, Abra:

1. Converts the area and fill arguments into `Area` and `Fill` values.
2. Builds a shader for the selected fill style.
3. Rasterizes the closed area with anti-aliasing.
4. Applies feathering when the area has a non-zero feather radius.
5. Composites the result with source-over blending.

An empty or non-positive area returns a `1 x 1` image from `fill`. Use valid positive dimensions when creating rectangular areas.
