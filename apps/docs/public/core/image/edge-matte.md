---
title: Guided edge matting
order: 30
outline: deep
---

# Guided edge matting

`edge_matte` refines a rough foreground mask with an AI model that accepts a guide mask. It grows the rough mask, builds an unknown band around its edge, enlarges the relevant crop, and processes overlapping tiles one at a time. Only the center of each tile is copied into the result, which is returned as one photo-sized mask.

```rust
use abra_ai_core::{edge_matte, EdgeMatteOptions};

let refined = edge_matte(&model, &photo, &rough_mask, EdgeMatteOptions::default(), |_| {})?;
```

The model must return a mask and accept a guide. `None` is returned when the rough mask has no visible pixels. The progress callback receives the fraction of tiles completed.

`EdgeMatteOptions` defaults to an unknown band of `0.012` times the photo width, a `1.5` scale, tiles up to `1024` pixels on each side, and `128` pixels of overlap. Use its `with_unknown_band`, `with_scale`, `with_tile_size`, and `with_overlap` methods to adjust them. Tile inference is sequential; tile masks are assembled into a single buffer instead of allocating a full-photo mask for every tile.
