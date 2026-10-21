---
title: Edge filters
order: 6
outline: deep
---

# Edge filters

Edge filters detect or emphasize rapid changes in image intensity.

## Sobel operators

Use `SobelDirection` to choose the horizontal or vertical 3x3 gradient kernel. The builder supports `with_options` for area and mask scoping:

```rust
use abra::filters::prelude::sobel::{SobelDirection, sobel};

sobel(SobelDirection::Horizontal).apply(&mut image);
sobel(SobelDirection::Vertical).apply(&mut image);
```

Use a copy when both directional results are needed separately:

```rust
let mut horizontal = image.clone();
let mut vertical = image.clone();
sobel(SobelDirection::Horizontal).apply(&mut horizontal);
sobel(SobelDirection::Vertical).apply(&mut vertical);
```

## Color edges

`color_edges` draws a line wherever the color of an image changes, as the outline of a comic book is drawn between the flat areas of color. A pixel is on a line when its color differs from the pixel a thickness to the left of it, or the one a thickness above it, by more than the tolerance. The result is ink on paper, black on white unless you set them:

```rust
use abra::filters::prelude::edges::color_edges;

let mut lines = image.clone();
color_edges(3).with_tolerance(1).apply(&mut lines);
// Multiply the lines over the picture to outline it.
blend(&lines).with_mode(BlendMode::Multiply).apply(&mut image);
```

The difference between two colors is the mean difference of their red, green and blue levels, from `0` to `255`, and `with_tolerance` sets how much counts. The default of 16 is clearly different. A picture of flat colors, where any change is an edge, wants a small one such as `1`, and a photograph, where grain makes small changes everywhere, wants a larger one. The lines are as wide as the thickness and sit on the right of an upright change and below a flat one.

Unlike a Sobel of the brightness, it looks at the colors, so a red next to a green of the same brightness is an edge. `with_ink` and `with_paper` set the colors, and the alpha of the image is kept. It runs on the GPU as well as the CPU, and the two draw the same lines.

## Glowing edges

`glowing_edges` builds a soft glowing-edge effect:

```rust
use abra::filters::prelude::edges::glowing_edges;

glowing_edges().apply(&mut image);
glowing_edges().with_edge_width(4).with_edge_brightness(10).with_smoothness(3).apply(&mut image);
```

Edge width defaults to `2`, edge brightness to `6`, and smoothness to `5`. The current implementation uses grayscale conversion, horizontal Sobel processing, kernel expansion, and blur; the brightness and smoothness parameters are currently reserved in the implementation.

## Edge workflow

A common workflow is to preserve the original image, derive edges from a copy, and then use the result as a mask or overlay:

```rust
let mut edges = image.clone();
sobel(SobelDirection::Horizontal).apply(&mut edges);
edges.write("out/edges.png", None)?;
```
