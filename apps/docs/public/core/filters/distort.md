---
title: Distort filters
order: 3
outline: deep
---

# Distort filters

Distort filters remap source coordinates instead of only changing a pixel's color. They are useful for lens-like shaping and wave effects. See [Apply options](../apply-options) for regional processing.

## Pinch and bulge

`pinch` applies a radial distortion around the image center:

```rust
use abra::filters::prelude::distort::pinch;

pinch(&mut image, 0.45, None);
pinch(&mut image, -0.35, None);
```

Positive amounts pinch inward; negative amounts bulge outward. The amount is clamped to `-1.0..=1.0`.

## Displace

`displace` moves pixels by the brightness of another image, like Photoshop's Displace filter. A height map bends whatever is drawn on the image so it follows the map:

```rust
use abra::filters::prelude::distort::displace;

displace(&map, 0.0, 40.0).apply(&mut image);
```

Where the map is mid gray nothing moves. Where it is white the content moves by the full amount along each axis, and where it is black it moves the same distance the other way. The amounts are in pixels; a positive `amount_x` moves content right and a positive `amount_y` moves it down where the map is bright. A map of a different size is stretched over the image, and pixels pulled in from past an edge take the nearest edge pixel.

## Ripple

`ripple` displaces pixels with a periodic wave:

```rust
use abra::filters::prelude::distort::{RippleShape, RippleSize, ripple};

ripple(0.6).apply(&mut image);
ripple(0.6).with_size(RippleSize::Large).with_shape(RippleShape::Angle(45.0)).apply(&mut image);
```

`with_size` takes a `RippleSize`, which controls wavelength. It defaults to `Medium`:

- `Small`: fewer, wider ripples.
- `Medium`: middle wavelength.
- `Large`: tighter, more frequent ripples.

`with_shape` takes a `RippleShape`, which controls the displacement pattern. It defaults to `Circular`:

- `Circular`: radial waves from the center.
- `Square`: square-distance waves.
- `Angle(degrees)`: directional waves; `0` is vertical and `90` is horizontal.
- `Random`: deterministic random displacement per pixel.

The ripple amount is clamped to `-1.0..=1.0`. Positive amounts create inward ripples and negative amounts create outward ripples.
