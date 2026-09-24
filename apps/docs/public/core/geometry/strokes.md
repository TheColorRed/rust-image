---
title: Strokes
order: 5
outline: deep
---

# Strokes

A stroke expands a `Path` or `Area` boundary by a specified width. The result is geometry that can be filled, composited, or used as another shape.

## Stroke a path

```rust
use abra::abra_core::{LineCap, LineJoin, Path};

let line = Path::line((20, 80), (300, 80));
let outline = line.stroke(16.0).with_join(LineJoin::Round).with_cap(LineCap::Round).to_path();
```

The width is centered on the original path. `stroke` returns a builder: set the cap, join, and miter limit, then build the outline with `to_path()` (or `into()` wherever a `Path` or `Area` is expected). Without any settings the stroke uses `Miter` joins and `Butt` caps.

## Line caps

`LineCap` controls how an open path ends:

| Cap | Behavior |
| --- | --- |
| `Butt` | Ends exactly at the endpoint. |
| `Round` | Adds a rounded endpoint. |
| `Square` | Extends a square endpoint beyond the path. |

```rust
let butt = line.stroke(12.0).to_path();
let round = line.stroke(12.0).with_cap(LineCap::Round).to_path();
let square = line.stroke(12.0).with_cap(LineCap::Square).to_path();
```

Caps affect open-path endpoints. Closed areas do not have exposed endpoints.

## Line joins

`LineJoin` controls how adjacent segments meet at corners:

| Join | Behavior |
| --- | --- |
| `Miter` | Creates a pointed corner. |
| `Round` | Adds a rounded corner. |
| `Bevel` | Cuts the corner off. |

```rust
let pointed = line.stroke(12.0).to_path();
let rounded = line.stroke(12.0).with_join(LineJoin::Round).to_path();
let beveled = line.stroke(12.0).with_join(LineJoin::Bevel).to_path();
```

A miter on a sharp corner can reach far past the line. When it would reach more than the miter limit times half the width, the corner is beveled instead. The limit defaults to `4.0`:

```rust
let spiky = line.stroke(12.0).with_miter_limit(10.0).to_path();
```

## Fill a stroke

Stroke geometry is still geometry, so use `fill` to rasterize it:

```rust
use abra::abra_core::Color;
use abra::drawing::prelude::fill;

let image = fill(rounded, Color::from_rgba(30, 120, 220, 255)).to_image();
```

For an area boundary, the stroked result is a closed area:

```rust
use abra::abra_core::{Area, Color, LineJoin};

let shape = Area::circle((100, 100), 80.0);
let border = shape.stroke(10.0).with_join(LineJoin::Round).to_area();
let border_image = fill(border, Color::white()).to_image();
```

## Width and curves

Curved paths are flattened internally to construct smooth stroke geometry. Use a positive width and choose round joins and caps when a softer outline is needed.

```rust
let mut curve = Path::new();
curve.move_to((20, 120))
  .cubic_to((80, 20), (220, 220), (300, 120));

let smooth = curve.stroke(10.0).with_join(LineJoin::Round).with_cap(LineCap::Round).to_path();
```
