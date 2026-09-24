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
