---
title: Sprites and trails
order: 6
outline: deep
---

# Sprites and trails

`Sprites` is the stamping engine: it draws many small soft dots, textures and lines over an image, on the GPU when there is one and on the CPU otherwise, with the same pixels either way. Brush dabs and strokes, the stroke overlay and the particle system all draw through it. Something else works out where things go; `Sprites` only draws them.

`Sprites` is an abra effect, so it has `apply`, `applied`, `with_area` and `with_mask` like every other effect (see [Apply options](../apply-options)). It needs nothing but the library: it works headless, with no app, window or GPU.

## Draw sprites

A `Sprite` is a dot, or a texture, at a place in the image. The place and size are in pixels of the image.

```rust
use abra::drawing::prelude::{Blend, Sprite, sprites};
use abra::prelude::*;

let mut image = Image::read("input.png")?;

let dots = (0..40).map(|i| {
  Sprite::new(30.0 + i as f32 * 12.0, 80.0, 6.0)
    .with_color((255, 120, 40))
    .with_hardness(0.0)
    .with_opacity(0.8)
    .with_blend(Blend::Additive)
});
sprites().with_sprites(dots.collect()).apply(&mut image);
```

| Function                              | Effect                                                                                                |
| ------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| `Sprite::new(x, y, size)`             | A white, opaque dot of radius `size` (half the longer side of a texture) at `x`, `y`.                 |
| `with_color((r, g, b))`               | The color of a dot, or the tint of a texture.                                                         |
| `with_opacity(value)`                 | How opaque the sprite is, `0.0` to `1.0`.                                                             |
| `with_hardness(value)`                | How hard a dot's edge is, as a [brush's hardness](./brushes). A texture does not use it.              |
| `with_rotation(degrees)`              | Turns the sprite around its middle.                                                                   |
| `with_blend(blend)`                   | A `Blend`: any blend mode, or `Blend::Additive` to add light.                                         |
| `with_texture(image)`                 | Draws the image instead of a dot, scaled so its longer side is twice `size`, bilinearly sampled.      |
| `with_stretch(amount)`                | Makes the sprite longer along the way it is turned: 0 is as long as wide, 1 is twice as long. A dot becomes an oval. |
| `with_frame(columns, rows, frame)`    | Draws one frame of a texture that is a grid of frames, counting along each row and then down. Frames are cut at whole pixels and not mixed with their neighbors. |
| `with_color_from_image_at(x, y)`      | Takes the color from the image at that point, read by the renderer so the pixels stay in the library. |

Sprites are drawn in the order they are given, so a later one is over an earlier one.

## Draw trails

A `SpriteTrail` is a line through points from its tail to its head. Each `TrailPoint` has its own width, opacity and color, which change evenly between points, so a trail can taper, fade and change color along its length.

```rust
use abra::drawing::prelude::{SpriteTrail, TrailPoint, sprites};

let points = vec![
  TrailPoint::new(20.0, 120.0, 14.0).with_opacity(0.0),
  TrailPoint::new(90.0, 60.0, 8.0).with_opacity(0.6).with_color((255, 200, 80)),
  TrailPoint::new(180.0, 30.0, 2.0).with_opacity(1.0).with_color((255, 255, 255)),
];
sprites().with_trails(vec![SpriteTrail::new(points).with_hardness(0.5)]).apply(&mut image);
```

A pixel takes its color from the closest place on the line. Trails are drawn first, under all the sprites. A trail needs at least two points.

## Thousands at once

When you work out thousands of sprites, making an object for each one is slow, and slower still across a language binding where every call has a cost. `add_sprite_values` and `add_trail_values` take the sprites and trails as flat lists of numbers instead, in one call, and draw exactly what the objects draw.

- A sprite is 16 numbers: `x`, `y`, `size`, `opacity`, `red`, `green`, `blue` (0 to 255), `hardness`, `rotation`, then `1` and a point `x`, `y` when its color is read from the image there (`0`, `0`, `0` when it is not), then `stretch`, and the `columns`, `rows` and `frame` of a texture that is a grid of frames (`1`, `1`, `0` when it is not).
- A trail point is 7 numbers: `x`, `y`, `width`, `opacity`, `red`, `green`, `blue`. `add_trail_values` also takes how many points each trail has.
- The sprites in one call share a blend and a texture, and the trails share a hardness and a blend. Call it again for another group.
- A list that is not a whole number of sprites, or lengths that do not match the points, panics.

## Hardness

Hardness is the same for dots, trails and brushes: `0.0` fades like a quadratic from the middle, `0.5` fades in a straight line, and `1.0` is solid all the way to the edge with about a pixel of anti-aliasing, so a radius 6 dot at hardness `1.0` is 12 pixels across.

## Zoomed views

The shader maps each pixel back to the photo, so a zoomed-out view of the result shows the same sprites smaller. This is what lets a live view draw a stroke or particles at any zoom without drawing them again.

## Related

- [Brushes](./brushes): dabs and strokes with a solid color are drawn as sprites and trails.
- [Random numbers, noise and ranges](../utilities#random-numbers-noise-and-ranges): `Random`, `value_noise` and `CurveRange`, which the particle system uses to place sprites.
