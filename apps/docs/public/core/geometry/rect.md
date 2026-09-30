---
title: Rectangles and viewports
order: 7
outline: deep
---

# Rectangles and viewports

`Rect` is an upright rectangle. It is what every `bounds()` call returns, and it is the coordinate system geometry is mapped out of, the role SVG's `viewBox` plays.

## Creating rectangles

```rust
use abra::abra_core::{PointF, Rect, Size};

let from_origin = Rect::new((0, 0), (320, 180));
let from_edges = Rect::from_edges(10, 20, 110, 70);
let around = Rect::from_points([(3.0, 9.0), (-1.0, 2.0), (7.0, 4.0)]);
let image_frame: Rect = Size::new(1920, 1080).into();
```

A width or height of zero or less makes an empty rectangle; check with `is_empty`.

## Reading edges

```rust
let rect = Rect::new((10, 20), (100, 50));
let (left, top, right, bottom) = rect.edges::<i32>();
let center = rect.center();
let corners = rect.corners(); // top-left, top-right, bottom-right, bottom-left
```

`edges` converts to any numeric type; integer types are rounded. `left`, `top`, `right`, `bottom`, `origin`, and `size` read single values.

## Clipping

`intersect` returns the overlap of two rectangles, or an empty rectangle when they do not overlap. Clip a shape's bounds to an image before iterating its pixels:

```rust
use abra::abra_core::ImageExt;

let visible = area.bounds().intersect(Rect::new((0, 0), image.size()));
if !visible.is_empty() {
  let (min_x, min_y, max_x, max_y) = visible.edges::<u32>();
}
```

`contains` tests a point; the left and top edges are inside, the right and bottom edges are not.

## Mapping between coordinate systems

`map_point` maps a point from a rectangle's coordinate system into a viewport of a given size, following an `AspectRatio` policy:

```rust
use abra::abra_core::{AspectRatio, PointF, Rect, Size};

let design = Rect::new((0, 0), (100, 100));
let point = design.map_point(PointF::new(50.0, 50.0), Size::new(800, 600), AspectRatio::meet());
```

Paths and areas use the same mapping. `transform_to_viewport` maps a whole path out of an explicit coordinate system, and `fit` uses the path's own bounds:

```rust
use abra::abra_core::{Area, AspectRatio, Rect, Shape};

let heart = Area::shape(Shape::Heart);
let design = Rect::new((0, 0), (100, 120));
let rendered = heart.transform_to_viewport(&design, (500, 600), AspectRatio::meet());
let icon = heart.fit((32, 32), AspectRatio::meet());
```

The original geometry is not modified, so one shape can be rendered at many sizes.

## Aspect ratio policies

`AspectRatio` combines a scaling mode with horizontal and vertical alignment:

```rust
use abra::abra_core::{Alignment, AspectRatio, PreserveAspectRatio};

let stretch = AspectRatio::none();
let centered_fit = AspectRatio::meet();
let centered_cover = AspectRatio::slice();
let top_left_fit = AspectRatio::new(PreserveAspectRatio::Meet, Alignment::Min, Alignment::Min);
```

| Policy                 | Behavior                                                           |
| ---------------------- | ------------------------------------------------------------------ |
| `AspectRatio::none()`  | Stretch to fill the viewport.                                      |
| `AspectRatio::meet()`  | Preserve the ratio and fit inside; empty space may remain.         |
| `AspectRatio::slice()` | Preserve the ratio and cover the viewport; content may be cropped. |

When using `Meet` or `Slice`, alignment controls where extra space or cropped content is placed:

- `Alignment::Min`: left or top.
- `Alignment::Mid`: centered.
- `Alignment::Max`: right or bottom.
