---
title: Image Overview
order: 0
outline: deep
---

# Image Overview

`Image` is Abra's central raster type. It stores RGBA pixels, supports file I/O, and is the target for adjustments, filters, transforms, masks, and drawing.

## Image guides

| Guide                            | Covers                                                                        |
| -------------------------------- | ----------------------------------------------------------------------------- |
| [Image operations](./operations) | Construct images, inspect dimensions, access pixels, and update RGBA data.    |
| [Loading and saving](./io)       | Read supported image formats, write output files, and load image collections. |

Most image-processing APIs mutate an `Image` in place. Use `clone` or `as_image` when a separate result is needed.
