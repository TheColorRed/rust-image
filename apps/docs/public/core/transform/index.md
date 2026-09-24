---
title: Transform Overview
order: 0
outline: deep
---

# Transform Overview

Transforms change image dimensions, crop pixels, rotate content, or flip an image. The traits are re-exported through `abra::transform::prelude` and are also available in `abra::prelude`.

## Transform guides

| Guide                                  | Covers                                                    |
| -------------------------------------- | --------------------------------------------------------- |
| [Image transforms](./image-transforms) | Resize, crop, rotate, flip, and interpolation algorithms. |

Transforms mutate the target `Image` in place. Clone an image first when the source must remain unchanged.
