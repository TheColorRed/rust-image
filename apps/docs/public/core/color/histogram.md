---
title: Histogram
order: 5
outline: deep
---

# Histogram

`Histogram` counts the distribution of values in an image's red, green, blue, and alpha channels. Each channel has `256` bins, one for every possible `u8` value from `0` to `255`.

Use histograms for analysis, auto-levels, clipping, color statistics, and edge-aware operations.

## Creating a histogram

Create an empty histogram, or compute one from an image:

```rust
use abra::abra_core::{Histogram, Image};

let image = Image::from_path("assets/photo.png", None).unwrap();
let histogram = Histogram::from_image(&image);
```

To compute from packed RGBA bytes, pass a slice whose samples are ordered as `R, G, B, A`:

```rust
let pixels = [
  255, 0, 0, 255,
  0, 255, 0, 255,
  0, 0, 255, 128,
];

let histogram = Histogram::from_rgba(&pixels);
```

`from_image` and `from_rgba` count every pixel, including fully transparent pixels. Use the `_skip_transparent` variants to ignore samples whose alpha channel is `0`:

```rust
let visible_image_histogram = Histogram::from_image_skip_transparent(&image);
let visible_data_histogram = Histogram::from_rgba_skip_transparent(&pixels);
```

## Reading channel counts

Each channel getter returns a reference to a `[u64; 256]` array. The array index is the channel value and the array value is its count:

```rust
let red = histogram.red();
let green = histogram.green();
let blue = histogram.blue();
let alpha = histogram.alpha();

let number_of_red_255_pixels = red[255];
let pixels_counted = histogram.total_pixels();
```

`total_pixels` sums the red channel and therefore represents the number of samples counted. The channel arrays can be used directly for charts or custom analysis.

## Clearing and editing data

Reset an existing histogram with `clear`:

```rust
let mut histogram = Histogram::from_rgba(&pixels);
histogram.clear();
assert_eq!(histogram.total_pixels(), 0);
```

For low-level RGB edits, `rgb_mut` returns mutable references to the red, green, and blue arrays:

```rust
let mut histogram = Histogram::new();
let (red, green, blue) = histogram.rgb_mut();
red[128] += 1;
green[128] += 1;
blue[128] += 1;
```

The alpha channel is not included in `rgb_mut`.

## Clip bounds and levels LUTs

Clip bounds identify low and high channel values after removing a symmetric fraction from each tail of a distribution:

```rust
let (red_low, red_high) = histogram.red_clip_bounds(0.01);
let (green_low, green_high) = histogram.green_clip_bounds(0.01);
let (blue_low, blue_high) = histogram.blue_clip_bounds(0.01);
```

The clip fraction is applied independently to both tails. A value of `0.01` clips approximately one percent from the low end and one percent from the high end.

Generate a levels lookup table from those bounds:

```rust
let red_levels = histogram.red_levels_lut(0.01);
let mapped_red = red_levels[128];
```

Each LUT maps an input channel value from `0..=255` to an output value from `0..=255`. Values at or below the low bound map to `0`; values at or above the high bound map to `255`; values between them are scaled linearly.

Use `clip_bounds_from_slice` and `levels_lut_from_slice` when working with a custom channel array:

```rust
let custom_channel = [0u64; 256];
let bounds = histogram.clip_bounds_from_slice(&custom_channel, 0.01);
let lut = histogram.levels_lut_from_slice(&custom_channel, 0.01);
```

## Medians and means

Calculate RGB medians from the full histogram:

```rust
let (red_median, green_median, blue_median) = histogram.rgb_medians();
```

Or calculate a single channel using an explicit sample count:

```rust
let count = histogram.total_pixels();
let red_median = histogram.red_median(count);
let green_mean = histogram.green_mean(count);
let blue_mean = histogram.blue_mean(count);
```

The static helpers `median_from_hist` and `mean_from_hist` work with any `[u64; 256]` channel array. `mean_from_hist` returns `0` when the count is zero.

## Percentiles

Find a channel value at a normalized percentile. A percentile of `0.5` is commonly used as a median:

```rust
let count = histogram.total_pixels();
let red_p10 = histogram.red_percentile(count, 0.10);
let green_p50 = histogram.green_percentile(count, 0.50);
let blue_p90 = histogram.blue_percentile(count, 0.90);
```

The percentile is clamped to the `0.0..=1.0` range. For custom channel data, use `percentile_from_hist`.

## Weighted averages

A weighted average limits the calculation to values within a threshold of a center value. This is useful for edge-aware or bilateral-style processing:

```rust
let center = 128;
let threshold = 12;
let red_average = histogram.red_weighted_average(center, threshold);
let green_average = histogram.green_weighted_average(center, threshold);
let blue_average = histogram.blue_weighted_average(center, threshold);
```

If no values fall inside the range, the weighted-average functions return the supplied center value. The static `weighted_average_in_range` helper accepts a custom channel histogram.

## API summary

| API                                                          | Purpose                                       |
| ------------------------------------------------------------ | --------------------------------------------- |
| `Histogram::new()`                                           | Create an empty four-channel histogram.       |
| `from_image` / `from_rgba`                                   | Count all image or RGBA samples.              |
| `from_image_skip_transparent` / `from_rgba_skip_transparent` | Count only samples with non-zero alpha.       |
| `red`, `green`, `blue`, `alpha`                              | Read channel bins.                            |
| `total_pixels`                                               | Return the number of counted samples.         |
| `clear`                                                      | Reset all channel bins.                       |
| `rgb_mut`                                                    | Mutably access RGB channel bins.              |
| `*_clip_bounds`                                              | Calculate symmetric low/high clipping bounds. |
| `*_levels_lut`                                               | Build an 8-bit levels lookup table.           |
| `rgb_medians`, `*_median`                                    | Calculate median channel values.              |
| `*_mean`                                                     | Calculate mean channel values.                |
| `*_percentile`                                               | Read a value at a normalized percentile.      |
| `*_weighted_average`                                         | Average values within a channel threshold.    |
