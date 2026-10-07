---
title: Tools Overview
order: 0
outline: deep
---

# Tools Overview

Tools are interactive-style edits that work from points and areas you choose on an image, such as a spot to heal, a line to make level, or a rectangle to flatten. They are re-exported through `abra::tools::prelude`.

```rust
use abra::prelude::*;
use abra::tools::prelude::*;
```

## Tool guides

| Guide                        | Covers                                                                     |
| ---------------------------- | -------------------------------------------------------------------------- |
| [Remover](./remover)         | Heal blemishes, spots, and scratches by blending in the surrounding image. |
| [Straighten](./straighten)   | Rotate an image until a line you pick is level or plumb.                   |
| [Perspective](./perspective) | Flatten an angled rectangle, such as a building, sign, or page.            |
| [Skin](./skin)               | Smooth, tan, or adjust tone through a reusable detected skin mask.         |

## Applying a tool

Every tool is built with a function, adjusted with `with_*` builder methods, and run with `apply`, which edits the image in place:

```rust
let mut image = Image::read("assets/photo.jpg")?;

straighten(PointF::new(100, 950), PointF::new(600, 1050))
  .with_crop(true)
  .apply(&mut image);
```

`apply` comes from the `Tool` trait, so it must be in scope. Importing `abra::tools::prelude::*` does that. Clone the image first when the original must remain unchanged.

A tool that is built once can be reused. Tools that implement `Clone`, like the [Remover](./remover#reusing-a-stamp), can be kept as a template.
