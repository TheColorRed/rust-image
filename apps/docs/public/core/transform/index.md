---
title: Transform Overview
order: 0
outline: deep
---

# Transform Overview

Transforms change image dimensions, crop pixels, rotate content, or flip an image. `transform()` does all of these in one go; it is re-exported through `abra::transform::prelude` and is also available in `abra::prelude`.

## Transform guides

| Guide                                  | Covers                                                    |
| -------------------------------------- | --------------------------------------------------------- |
| [Image transforms](./image-transforms) | Resize, crop, rotate, flip, and interpolation algorithms. |

Transforms mutate the target `Image` in place. Clone an image first when the source must remain unchanged.
