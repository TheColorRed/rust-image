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

## Random numbers, noise and ranges

These are for placing things repeatably, such as particles. They are in `abra_core` and work without any app.

`Random` is a seeded generator whose numbers are fixed by Abra's own code (SplitMix64), not by a dependency, so the same seed gives the same numbers on every machine and in every version:

```rust
use abra::abra_core::Random;

let mut random = Random::new(42);
let x = random.range(0.0, 100.0);
let spot = random.point_in_circle((200.0, 150.0), 40.0);
let ring = random.point_in_ring((200.0, 150.0), 20.0, 40.0);
let bright = random.points_in_light(&image, 0.7, 500); // 500 points on the light parts of the image
```

It also has `float`, `floats(count)`, `index(len)`, `point_in_rect` and `point_on_line`.

`value_noise(x, y, seed)` is smooth noise from `-1.0` to `1.0` that changes slowly across `x` and `y`.

`CurveRange` is a value that follows a curve over a life, random between two curves. Make it from a number, a `(min, max)` pair, a `Curve` or a pair of curves. `at(age, pick)` reads the value at an age from `0.0` to `1.0`, where `pick` chooses a place between the curves, and `area(age, pick)` adds a velocity up from birth.

## Public helper types

Other small shared types are documented with the feature that owns them:

- `Channels` and pixel storage: [Image operations](./image/operations)
- `WriterOptions`: [Loading and saving](./image/io)
- `TransformAlgorithm`: [Image transforms](./transform/image-transforms)
- `ImageFormat`: [Loading and saving](./image/io)
