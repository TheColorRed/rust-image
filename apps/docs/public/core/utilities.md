---
title: Core utilities
order: 9
outline: deep
---

# Core utilities

The core crate exposes a few small conversion and selection utilities. They are mainly useful when extending Abra or integrating its lower-level modules.

## `FromF32`

`FromF32` converts a floating-point value to selected numeric types:

```rust
use abra::abra_core::FromF32;

let exact = f32::from_f32(1.25);
let rounded = i32::from_f32(1.6);
let channel = u8::from_f32(260.0);
```

The integer implementations round. `u8` and `u32` clamp to their supported ranges.

## `if_pick!`

`if_pick!` expands a compact if/else-if/else selection expression:

```rust
use abra::if_pick;

let scale = if_pick!(radius >= 96 => 8, radius >= 48 => 4, else => 2);
```

This macro is primarily intended for implementation code that selects a value from ordered conditions.

## Public helper types

Other small shared types are documented with the feature that owns them:

- `Channels` and pixel storage: [Image operations](./image/operations)
- `WriterOptions`: [Loading and saving](./image/io)
- `TransformAlgorithm`: [Image transforms](./transform/image-transforms)
- `FileInfo`: [Loading and saving](./image/io)
