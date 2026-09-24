---
title: Shapes
order: 6
outline: deep
---

# Shapes

Abra includes reusable closed shapes that return `Area` values. They are defined in a normalized coordinate space and can be filled, stroked, fitted, or transformed like any other area.

## Heart

`Heart::new` creates a heart using cubic Bezier curves:

```rust
use abra::abra_core::Heart;

let heart = Heart::new();
```

The default heart occupies approximately `0..=100` horizontally and `0..=120` vertically.

## Polygon

`Polygon::new` creates a regular polygon with the requested number of sides:

```rust
use abra::abra_core::Polygon;

let triangle = Polygon::new(3);
let hexagon = Polygon::new(6);
let decagon = Polygon::new(10);
```

A polygon must have at least three sides. Values below three panic because they cannot define a closed polygon.

The polygon is generated around a radius of `50` and occupies approximately a `100 x 100` coordinate space.

## Star

`Star::new` creates a five-pointed star from line segments:

```rust
use abra::abra_core::Star;

let star = Star::new();
```

The default star occupies a `100 x 100` coordinate space.

## Fill a shape

All built-in shapes return `Area`, so they work directly with `fill`:

```rust
use abra::abra_core::{Color, Heart, Star};
use abra::drawing::prelude::fill;

let heart_image = fill(Heart::new(), Color::red()).to_image();
let star_image = fill(Star::new(), Color::yellow()).to_image();
```

## Resize a shape

Use `fit`, `fit_square`, `fit_with_aspect`, `stretch`, or `cover` to place a normalized shape into an output size:

```rust
use abra::abra_core::{AspectRatio, Heart, Size};

let heart = Heart::new();
let contained = heart.fit(Size::new(400, 300));
let square = heart.fit_square(256.0);
let stretched = heart.stretch((400, 300));
let covered = heart.cover((400, 300));
let aligned = heart.fit_with_aspect(Size::new(400, 300), AspectRatio::meet());
```

Use `fit` or `fit_with_aspect` to preserve proportions. Use `stretch` when filling the exact viewport is more important than preserving the original ratio.

## Stroke a shape

```rust
use abra::abra_core::{LineJoin, Star};

let outline = Star::new().stroke(6.0).with_join(LineJoin::Round).to_area();
```

See [Strokes](./strokes) for join behavior.
