---
title: Shapes
order: 6
outline: deep
---

# Shapes

`Area::shape` creates a predefined closed shape. Shapes are drawn inside a `100 x 100` box at the origin and can be filled, stroked, fitted, or transformed like any other area.

```rust
use abra::abra_core::{Area, Shape};

let heart = Area::shape(Shape::Heart);
let star = Area::shape(Shape::Star);
let hexagon = Area::shape(Shape::Polygon(6));
```

| `Shape`       | Outline                                                                                   |
| ------------- | ----------------------------------------------------------------------------------------- |
| `Heart`       | A heart made of cubic Bezier curves. It is 120 tall, so it overflows the box at the bottom. |
| `Star`        | A five-pointed star.                                                                      |
| `Polygon(n)`  | A regular polygon with `n` sides and a corner at the top. Fewer than three sides produces an empty area. |

## Fill a shape

Shapes are `Area` values, so they work directly with `fill`:

```rust
use abra::abra_core::{Area, Color, Shape};
use abra::drawing::prelude::fill;

let heart_image = fill(Area::shape(Shape::Heart), Color::red()).to_image();
let star_image = fill(Area::shape(Shape::Star), Color::yellow()).to_image();
```

## Resize a shape

`fit` scales a shape from its own bounds into a target size. The `AspectRatio` decides how differing proportions are handled:

```rust
use abra::abra_core::{Area, AspectRatio, Shape, Size};

let heart = Area::shape(Shape::Heart);
let contained = heart.fit(Size::new(400, 300), AspectRatio::meet());
let square = heart.fit((256, 256), AspectRatio::meet());
let stretched = heart.fit((400, 300), AspectRatio::none());
let covered = heart.fit((400, 300), AspectRatio::slice());
```

Use `AspectRatio::meet()` to preserve proportions. Use `AspectRatio::none()` when filling the exact size matters more than the original ratio. See [Rectangles and viewports](./rect) for alignment.

## Stroke a shape

```rust
use abra::abra_core::{Area, LineJoin, Shape};

let outline = Area::shape(Shape::Star).stroke(6.0).with_join(LineJoin::Round).to_area();
```

See [Strokes](./strokes) for join behavior.
