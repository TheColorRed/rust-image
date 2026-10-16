---
title: Grow and shrink
order: 8
outline: deep
---

# Grow and shrink

`dilate` makes the bright parts of an image grow and `erode` makes them shrink, by a number of pixels, each of the red, green and blue channels on its own. The alpha is left as it was. The window is a square, and the edges of the image are not eaten.

On a picture of a mask (white where it reaches) they move the edge of the region out or in. For a `Mask` itself use `Mask::dilated` and `Mask::eroded`, which do the same on the mask's values (see [Masks](../mask/masks)).

```rust
use abra::filters::prelude::morphology::{dilate, erode};

dilate(3).apply(&mut image);
erode(3).apply(&mut image);
```

Both are effects, so they accept [apply options](../apply-options) and run on the GPU when it is available, giving the same pixels as the CPU. A radius of `0` changes nothing.
