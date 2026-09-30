---
title: Named colors
order: 2
outline: deep
---

# Named colors

Abra provides named constructors on `Color` for common opaque colors. Each constructor returns a `Color` with alpha `255`.

```rust
use abra::abra_core::Color;

let warning = Color::orange();
let link = Color::royal_blue();
let transparent = Color::transparent();
```

## Special colors

| Constructor            | RGBA                               |
| ---------------------- | ---------------------------------- |
| `Color::transparent()` | `(0, 0, 0, 0)`                     |
| `Color::random()`      | Random RGB values with alpha `255` |

`random` generates a new color each time it is called. It is useful for previews, placeholders, and randomized effects, but should not be used when a stable color is required.

## Red and pink

| Constructor         | RGBA                   |
| ------------------- | ---------------------- |
| `Color::red()`      | `(255, 0, 0, 255)`     |
| `Color::crimson()`  | `(220, 20, 60, 255)`   |
| `Color::ruby()`     | `(224, 17, 95, 255)`   |
| `Color::pink()`     | `(255, 192, 203, 255)` |
| `Color::magenta()`  | `(255, 0, 255, 255)`   |
| `Color::hot_pink()` | `(255, 105, 180, 255)` |

## Green

| Constructor             | RGBA                 |
| ----------------------- | -------------------- |
| `Color::green()`        | `(0, 255, 0, 255)`   |
| `Color::lime_green()`   | `(50, 205, 50, 255)` |
| `Color::sea_green()`    | `(46, 139, 87, 255)` |
| `Color::forest_green()` | `(34, 139, 34, 255)` |

## Blue

| Constructor           | RGBA                   |
| --------------------- | ---------------------- |
| `Color::blue()`       | `(0, 0, 255, 255)`     |
| `Color::royal_blue()` | `(65, 105, 225, 255)`  |
| `Color::sky_blue()`   | `(135, 206, 235, 255)` |
| `Color::navy_blue()`  | `(0, 0, 128, 255)`     |

## Warm and spectral colors

| Constructor       | RGBA                   |
| ----------------- | ---------------------- |
| `Color::yellow()` | `(255, 255, 0, 255)`   |
| `Color::orange()` | `(255, 165, 0, 255)`   |
| `Color::indigo()` | `(75, 0, 130, 255)`    |
| `Color::violet()` | `(238, 130, 238, 255)` |
| `Color::purple()` | `(128, 0, 128, 255)`   |
| `Color::tan()`    | `(210, 180, 140, 255)` |

## Neutral colors

| Constructor      | RGBA                   |
| ---------------- | ---------------------- |
| `Color::white()` | `(255, 255, 255, 255)` |
| `Color::black()` | `(0, 0, 0, 255)`       |
| `Color::gray()`  | `(128, 128, 128, 255)` |

## Custom alpha

Named colors are opaque. To use one with a custom alpha value, read its RGB channels and construct a new color:

```rust
use abra::abra_core::Color;

let red = Color::red();
let translucent_red = Color::from_rgba(red.r, red.g, red.b, 128);
```

For arbitrary colors, use [`Color::from_rgb`](./color) or [`Color::from_rgba`](./color).
