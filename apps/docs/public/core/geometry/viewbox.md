---
title: ViewBox
order: 7
outline: deep
---

# ViewBox

`ViewBox` defines an abstract coordinate system for paths and areas, similar to SVG's `viewBox`. Define geometry once in a logical space, then map it to any viewport size.

## Creating a viewbox

```rust
use abra::abra_core::ViewBox;

let explicit = ViewBox::new(0.0, 0.0, 100.0, 100.0);
let dimensions = ViewBox::from_dimensions(320.0, 180.0);
let square = ViewBox::square(100.0);
let unit = ViewBox::unit();
```

Use `unit` for normalized coordinates and `square` for common icon or shape coordinate systems.

## Scaling

Calculate independent or uniform scale factors:

```rust
let viewbox = ViewBox::new(0.0, 0.0, 100.0, 100.0);
let (scale_x, scale_y) = viewbox.scale_to_fit(800.0, 600.0);
let uniform = viewbox.uniform_scale_to_fit(800.0, 600.0);
let ratio = viewbox.aspect_ratio();
```

`scale_to_fit` permits non-uniform stretching. `uniform_scale_to_fit` returns the smaller factor so the whole viewbox fits inside the viewport.

## Mapping points

`map_point` transforms a `PointF` according to an `AspectRatio` policy:

```rust
use abra::abra_core::{AspectRatio, PointF, ViewBox};

let viewbox = ViewBox::new(0.0, 0.0, 100.0, 100.0);
let point = PointF::new(50.0, 50.0);
let viewport_point = viewbox.map_point(&point, 800.0, 600.0, AspectRatio::meet());
```

## Aspect ratio policies

`AspectRatio` combines a scaling mode with horizontal and vertical alignment:

```rust
use abra::abra_core::{Alignment, AspectRatio, PreserveAspectRatio};

let stretch = AspectRatio::none();
let centered_fit = AspectRatio::meet();
let centered_cover = AspectRatio::slice();
let top_left_fit = AspectRatio::new(
  PreserveAspectRatio::Meet,
  Alignment::Min,
  Alignment::Min,
);
```

| Policy | Behavior |
| --- | --- |
| `AspectRatio::none()` | Stretch to fill the viewport. |
| `AspectRatio::meet()` | Preserve the ratio and fit inside; empty space may remain. |
| `AspectRatio::slice()` | Preserve the ratio and cover the viewport; content may be cropped. |

When using `Meet` or `Slice`, alignment controls where extra space or cropped content is placed:

- `Alignment::Min`: left or top.
- `Alignment::Mid`: centered.
- `Alignment::Max`: right or bottom.

## Transforming paths and areas

Map an entire path or area into a viewport:

```rust
use abra::abra_core::{AspectRatio, Heart, ViewBox};

let heart = Heart::new();
let viewbox = ViewBox::new(0.0, 0.0, 100.0, 120.0);
let rendered = heart.transform_to_viewport(&viewbox, 500.0, 600.0, AspectRatio::meet());
```

The original geometry is not modified. The transformed path is a new value, so the same source shape can be reused at multiple sizes.

## Inferring a viewbox

Create a viewbox from a path or area's bounds:

```rust
let viewbox = heart.to_viewbox();
let icon_32 = heart.transform_to_viewport(&viewbox, 32.0, 32.0, AspectRatio::meet());
let icon_256 = heart.transform_to_viewport(&viewbox, 256.0, 256.0, AspectRatio::meet());
```

This is useful when geometry is authored at an arbitrary scale and should be normalized for responsive rendering.

## SVG-style rendering pattern

```rust
fn render_at_size(
  path: &abra::abra_core::Path,
  viewbox: &abra::abra_core::ViewBox,
  width: f32,
  height: f32,
) -> abra::abra_core::Path {
  path.transform_to_viewport(
    viewbox,
    width,
    height,
    abra::abra_core::AspectRatio::meet(),
  )
}
```

Define the path once, keep the viewbox with it, and create a transformed path only for the current output viewport.
