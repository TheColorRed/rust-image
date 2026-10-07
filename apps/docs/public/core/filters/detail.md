---
title: Detail filters
order: 5
outline: deep
---

# Detail filters

Detail filters increase local edge definition or reduce small variations. See [Apply options](../apply-options) for regional processing.

## Sharpen

`sharpen` increases local detail:

```rust
use abra::filters::prelude::sharpen::sharpen;

sharpen().apply(&mut image);
```

## Smooth

`smooth` reduces local variation across an image or selected area:

```rust
use abra::filters::prelude::smooth::smooth;

smooth().apply(&mut image);
```

## Choosing a detail filter

- Use `sharpen` to restore or emphasize local detail.
- Use `smooth` for general softening.
- Use [Skin tools](../../tools/skin) to smooth, tan, or adjust detected skin while protecting facial features.
- Use [Blur](./blur) for explicit blur kernels and focus effects.
