---
title: Histogram
order: 5
outline: deep
---

# Histogram

`Histogram` counts the distribution of values in an image's red, green, blue, and alpha channels. Each channel is a `Bins` value: `256` counts, one for every possible `u8` value from `0` to `255`, plus a running total.

Use histograms for analysis, auto-levels, clipping, color statistics, and edge-aware operations.

## Creating a histogram

Count the pixels of an RGBA buffer, such as an image's `rgba()`:

```rust
use abra::abra_core::{Channel, Histogram};
use abra::prelude::*;

let image = Image::read("assets/photo.png")?;
let histogram = Histogram::from_rgba(image.rgba(), false);
```

The second argument skips fully transparent pixels when `true`, so only visible pixels are counted:

```rust
let visible = Histogram::from_rgba(image.rgba(), true);
```

Counting runs in parallel across the image.

## Reading a channel

`channel` returns the `Bins` of one channel. `counts` is the raw `[u64; 256]` array, indexed by channel value:

```rust
let red = histogram.channel(Channel::R);
let number_of_red_255_pixels = red.counts()[255];
let pixels_counted = histogram.total_pixels();
```

## Statistics

Every statistic is a method on `Bins`, so it reads the same for any channel:

```rust
let green = histogram.channel(Channel::G);

let median = green.median();
let mean = green.mean();
let mode = green.mode();
let p10 = green.percentile(0.10);
let p90 = green.percentile(0.90);
```

`percentile` returns the value at that fraction of the sorted values: `0.0` is the smallest, `0.5` the median, and `1.0` the largest. `mode` returns the most frequent value, with ties going to the lowest. Empty bins return `0`.

## Clip bounds and levels LUTs

Clip bounds identify the low and high values after ignoring a fraction of the values at each end:

```rust
let (low, high) = histogram.channel(Channel::R).clip_bounds(0.01);
```

A value of `0.01` ignores about one percent at the low end and one percent at the high end.

Generate a levels lookup table from those bounds:

```rust
let red_levels = histogram.channel(Channel::R).levels_lut(0.01);
let mapped_red = red_levels[128];
```

Each LUT maps an input value from `0..=255` to an output value from `0..=255`. Values at or below the low bound map to `0`, values at or above the high bound map to `255`, and values between them are spread linearly.

## Weighted averages

A weighted average limits the calculation to values within a threshold of a center value. This is useful for edge-aware or bilateral-style processing:

```rust
let red_average = histogram.channel(Channel::R).weighted_average(128, 12);
```

If no values fall inside the range, the supplied center value is returned.

## Sliding windows

Filters that move a window across an image can keep one `Bins` per channel and update it as the window moves, instead of recounting. The running total keeps every statistic cheap:

```rust
use abra::abra_core::Bins;

let mut window = Bins::new();
window.add(40);
window.add(200);
window.add(90);
window.remove(200);
assert_eq!(window.median(), 90);
```

`Histogram::add_rgb(r, g, b)` counts the color channels of one pixel, and `clear` resets every channel.

## API summary

| API                                   | Purpose                                                |
| ------------------------------------- | ------------------------------------------------------ |
| `Histogram::from_rgba(px, skip)`      | Count an RGBA buffer, optionally skipping transparent. |
| `Histogram::new()`, `add_rgb`, `clear` | Build or reset a histogram by hand.                   |
| `channel(Channel)` / `channel_mut`    | Read or edit one channel's `Bins`.                     |
| `total_pixels`                        | The number of counted pixels.                          |
| `Bins::add`, `remove`, `merge`        | Update counts while keeping the running total.         |
| `median`, `mean`, `mode`, `percentile` | Channel statistics.                                   |
| `clip_bounds`, `levels_lut`           | Auto-levels bounds and lookup tables.                  |
| `weighted_average`                    | Average values within a threshold.                     |
