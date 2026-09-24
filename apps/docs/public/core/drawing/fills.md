---
title: Drawing fills
order: 1
outline: deep
---

# Drawing fills

Use `fill(..).to_image()` to rasterize a closed `Area` into a new image. Use `fill(..).apply(&mut image)` to create the filled result and composite it into an existing image.

## Fill an area

A solid color can be passed directly because `Color` converts into a `Fill`:

```rust
use abra::abra_core::{Area, Color};
use abra::drawing::prelude::fill;

let area = Area::circle((160, 100), 80.0);
let image = fill(area, Color::from_rgba(40, 120, 220, 255)).to_image();
image.write("out/circle.png", None)?;
```

## Fill with a gradient or image

All fill styles documented in [Color and Fill](../color/fill) are supported:

```rust
use abra::abra_core::{Area, Color, Gradient, Image, Star};
use abra::drawing::prelude::{fill, Path};

let gradient = Gradient::from_to(Color::purple(), Color::blue())
  .with_direction(Path::line((0, 0), (0, 200)));
let area = Area::rect((0, 0), (320, 200));
let image = fill(area, &gradient).to_image();
```

An `Image` can also be used as the fill source for an area:

```rust
let texture = Image::read("assets/texture.png")?;
let textured = fill(Star::new(), &texture).to_image();
```

Use the correct shape constructor for your geometry; the example above is illustrative for any closed `Area`.

## Fill an existing image

`apply` composites the generated fill into an existing image, and `with_position` sets where:

```rust
use abra::abra_core::{Area, Color, Image};
use abra::drawing::prelude::fill;

let mut canvas = Image::new(640, 360);
let area = Area::rect((0, 0), (200, 120));
fill(area, Color::from_rgba(255, 180, 40, 220)).with_position((80, 60)).apply(&mut canvas);
canvas.save("out/filled-image.png", None);
```

Without `with_position`, the area is drawn where it is. The position moves the generated fill image; it does not rewrite the area's geometry.

## Feathered edges

Area feathering softens coverage near the boundary:

```rust
let area = Area::circle((160, 100), 80.0).with_feather(16);
let soft = fill(area, Color::from_rgba(220, 60, 80, 255)).to_image();
```

The fill pipeline uses anti-aliased coverage and source-over compositing.

## API summary

| API                                       | Purpose                                          |
| ----------------------------------------- | ------------------------------------------------ |
| `fill(area, fill).to_image()`             | Return a new image containing a rasterized fill. |
| `fill(area, fill).apply(image)`           | Composite a fill into an existing image.         |
| `.with_position(point)`                   | Move where `apply` draws the fill.               |
| `Area::with_feather(pixels)`              | Soften fill coverage at an area's edge.          |
