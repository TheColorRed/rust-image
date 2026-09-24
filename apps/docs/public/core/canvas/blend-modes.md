---
title: Blend modes
order: 7
outline: deep
---

# Blend modes

Blend modes control how a layer's pixels combine with the pixels below it. Abra represents a mode as a function with the signature `fn(RGBA, RGBA) -> RGBA`, where the first color is the base and the second color is the layer being blended.

## Set a layer blend mode

Use a function from `abra::abra_core::blend` with `Layer::set_blend_mode`:

```rust
use abra::abra_core::blend;
use abra::canvas::prelude::*;

let canvas = Canvas::new("Blend");
canvas.add_layer_from_path("Background", "assets/background.jpg", None);
let overlay = canvas.add_layer_from_path("Overlay", "assets/overlay.png", None);

overlay.set_blend_mode(blend::multiply);
canvas.save("out/multiply.png", None);
```

Read the current function pointer with `blend_mode()` when a caller needs to retain or replace it.

## Configure a mode when adding a layer

Use `NewLayerOptions::with_blend_mode` to configure compositing at layer creation time:

```rust
use abra::abra_core::blend;
use abra::canvas::prelude::*;

let options = NewLayerOptions::new()
  .with_opacity(0.75)
  .with_blend_mode(blend::screen);

canvas.add_layer_from_path("Light leak", "assets/light.png", Some(options));
```

Layer opacity is clamped to `0.0..=1.0`. The blend function is applied during canvas composition.

## Blend mode groups

### Normal and selection

| Function        | Behavior                                                 |
| --------------- | -------------------------------------------------------- |
| `normal`        | Source-over compositing using the layer color and alpha. |
| `average`       | Averages the base and blend channels.                    |
| `darker_color`  | Selects the color with lower lightness.                  |
| `lighter_color` | Selects the color with higher lightness.                 |

### Darkening

| Function      | Behavior                                                            |
| ------------- | ------------------------------------------------------------------- |
| `darken`      | Selects the minimum value per channel.                              |
| `multiply`    | Multiplies base and blend channels, generally darkening the result. |
| `color_burn`  | Increases contrast while darkening based on the blend color.        |
| `linear_burn` | Darkens through linear channel subtraction.                         |
| `subtract`    | Subtracts the blend channels from the base.                         |

### Lightening

| Function       | Behavior                                                  |
| -------------- | --------------------------------------------------------- |
| `lighten`      | Selects the maximum value per channel.                    |
| `screen`       | Screens the colors, generally producing a lighter result. |
| `color_dodge`  | Brightens the base to reflect the blend color.            |
| `linear_dodge` | Adds the channel values to brighten the result.           |
| `glow`         | Produces a bright glow effect.                            |
| `reflect`      | Reflects the blend color to create a shiny effect.        |

### Contrast and lighting

| Function       | Behavior                                                       |
| -------------- | -------------------------------------------------------------- |
| `overlay`      | Combines multiply and screen behavior based on the base value. |
| `soft_light`   | Applies a softer contrast and lighting effect.                 |
| `hard_light`   | Applies a stronger contrast effect based on the blend value.   |
| `vivid_light`  | Combines color burn and color dodge behavior.                  |
| `linear_light` | Combines linear burn and linear dodge behavior.                |
| `pin_light`    | Selects or replaces channels based on the blend value.         |
| `hard_mix`     | Produces a high-contrast, threshold-like result.               |
| `phoenix`      | Produces a fiery glow with deep shadows and highlights.        |

### Difference and component modes

| Function        | Behavior                                                       |
| --------------- | -------------------------------------------------------------- |
| `difference`    | Calculates the absolute channel difference.                    |
| `exclusion`     | Produces a lower-contrast difference effect.                   |
| `divide`        | Divides base channels by blend channels.                       |
| `negation`      | Applies a negation-style color combination.                    |
| `grain_extract` | Extracts grain-like differences from the blend.                |
| `grain_merge`   | Merges grain-like differences into the base.                   |
| `hue`           | Uses the blend hue with base color information.                |
| `saturation`    | Uses blend saturation with base color information.             |
| `color`         | Uses blend hue and saturation while retaining base luminosity. |
| `luminosity`    | Uses blend luminosity with base color information.             |

## Blend two images directly

The blend module can combine images without creating a canvas:

```rust
use abra::abra_core::blend;
use abra::prelude::*;

let mut base = Image::read("assets/base.jpg")?;
let overlay = Image::read("assets/overlay.png")?;

blend::blend(&overlay).with_mode(blend::screen).apply(&mut base);
base.write("out/screened.png", None)?;
```

The source is placed at `(0, 0)` at full opacity with the `normal` mode unless you set otherwise. The destination
always starts at its own origin; only the source is positioned.

```rust
blend::blend(&overlay)
  .with_offset((80, 40))
  .with_opacity(0.6)
  .with_mode(blend::soft_light)
  .apply(&mut base);
```

The opacity is clamped to `0.0..=1.0` and is applied while compositing the blended result.

## Identify a blend mode

`blend_mode_name` returns a machine-readable identifier and display label:

```rust
let (id, label) = blend::blend_mode_name(blend::multiply);
assert_eq!(id, "multiply");
assert_eq!(label, "Multiply");
```

Unknown function pointers return `("unknown", "Unknown")`.

## Choosing a mode

- Use `normal` for standard layer compositing.
- Use `multiply`, `darken`, or `linear_burn` to deepen shadows.
- Use `screen`, `lighten`, or `linear_dodge` to add light.
- Use `overlay`, `soft_light`, or `hard_light` for contrast.
- Use `difference` or `exclusion` for visual comparisons.
- Use `color`, `hue`, `saturation`, or `luminosity` for component-based color work.
