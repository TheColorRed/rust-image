---
title: Drawing Overview
order: 0
outline: deep
---

# Drawing Overview

Abra drawing APIs rasterize fills, paths, areas, brush dabs, and brush strokes into images. Most application code can use the convenience functions and the `Painter` context.

## Drawing guides

| Guide                                       | Covers                                                                   |
| ------------------------------------------- | ------------------------------------------------------------------------ |
| [Fills](./fills)                            | Fill areas and images with solid colors, gradients, or image sources.    |
| [Brushes](./brushes)                        | Configure brush size, shape, fill, hardness, opacity, dabs, and strokes. |
| [Painter](./painter)                        | Use a mutable drawing context for repeated brush operations.             |
| [Rasterization primitives](./rasterization) | Build custom coverage, shader, compositor, and sampling pipelines.       |
| [SDF drawing helpers](./sdf)                | Use signed-distance helpers and optimized solid drawing functions.       |

Drawing works with [Geometry](../geometry/) and [Color](../color/) values. Use [Masks](../mask/) when coverage should be reusable across operations.
