---
title: Blend modes
order: 7
outline: deep
---

# Blend modes

Blend modes control how a layer's pixels combine with the pixels below it. Abra represents a mode as a `BlendMode` value. The mode computes a blend color from the base color underneath and the layer's color; the layer's alpha and opacity then composite that color over the base.

## Set a layer blend mode

Pass a `BlendMode` to `Layer::set_blend_mode`:

```rust
use abra::abra_core::BlendMode;
use abra::canvas::prelude::*;

let canvas = Canvas::new("Blend");
canvas.add_layer_from_path("Background", "assets/background.jpg", None);
let overlay = canvas.add_layer_from_path("Overlay", "assets/overlay.png", None);

overlay.set_blend_mode(BlendMode::Multiply);
canvas.save("out/multiply.png", None);
```

Read the current mode with `blend_mode()`.

## Configure a mode when adding a layer

Use `NewLayerOptions::with_blend_mode` to configure compositing at layer creation time:

```rust
use abra::abra_core::BlendMode;
use abra::canvas::prelude::*;

let options = NewLayerOptions::new().with_opacity(0.75).with_blend_mode(BlendMode::Screen);

canvas.add_layer_from_path("Light leak", "assets/light.png", Some(options));
```

Layer opacity is clamped to `0.0..=1.0`. The blend mode is applied during canvas composition.

## Blend mode groups

### Normal and selection

| Mode           | Behavior                                                 |
| -------------- | -------------------------------------------------------- |
| `Normal`       | Source-over compositing using the layer color and alpha. |
| `Average`      | Averages the base and blend channels.                    |
| `DarkerColor`  | Selects the color with lower lightness.                  |
| `LighterColor` | Selects the color with higher lightness.                 |

### Darkening

| Mode         | Behavior                                                            |
| ------------ | ------------------------------------------------------------------- |
| `Darken`     | Selects the minimum value per channel.                              |
| `Multiply`   | Multiplies base and blend channels, generally darkening the result. |
| `ColorBurn`  | Increases contrast while darkening based on the blend color.        |
| `LinearBurn` | Darkens through linear channel subtraction.                         |
| `Subtract`   | Subtracts the blend channels from the base.                         |

### Lightening

| Mode          | Behavior                                                  |
| ------------- | --------------------------------------------------------- |
| `Lighten`     | Selects the maximum value per channel.                    |
| `Screen`      | Screens the colors, generally producing a lighter result. |
| `ColorDodge`  | Brightens the base to reflect the blend color.            |
| `LinearDodge` | Adds the channel values to brighten the result.           |
| `Glow`        | Produces a bright glow effect.                            |
| `Reflect`     | Reflects the blend color to create a shiny effect.        |

### Contrast and lighting

| Mode          | Behavior                                                       |
| ------------- | -------------------------------------------------------------- |
| `Overlay`     | Combines multiply and screen behavior based on the base value. |
| `SoftLight`   | Applies a softer contrast and lighting effect.                 |
| `HardLight`   | Applies a stronger contrast effect based on the blend value.   |
| `VividLight`  | Combines color burn and color dodge behavior.                  |
| `LinearLight` | Combines linear burn and linear dodge behavior.                |
| `PinLight`    | Selects or replaces channels based on the blend value.         |
| `HardMix`     | Produces a high-contrast, threshold-like result.               |
| `Phoenix`     | Produces a fiery glow with deep shadows and highlights.        |

### Difference and component modes

| Mode           | Behavior                                                       |
| -------------- | -------------------------------------------------------------- |
| `Difference`   | Calculates the absolute channel difference.                    |
| `Exclusion`    | Produces a lower-contrast difference effect.                   |
| `Divide`       | Divides base channels by blend channels.                       |
| `Negation`     | Applies a negation-style color combination.                    |
| `GrainExtract` | Extracts grain-like differences from the blend.                |
| `GrainMerge`   | Merges grain-like differences into the base.                   |
| `Hue`          | Uses the blend hue with base color information.                |
| `Saturation`   | Uses blend saturation with base color information.             |
| `Color`        | Uses blend hue and saturation while retaining base luminosity. |
| `Luminosity`   | Uses blend luminosity with base color information.             |

## Custom modes

`BlendMode::Custom` wraps a function taking `(base, blend)` colors and returning the blend color:

```rust
use abra::abra_core::{BlendMode, blend::RGBA};

fn swap_red_blue(_base: RGBA, blend: RGBA) -> RGBA {
  (blend.2, blend.1, blend.0, blend.3)
}

overlay.set_blend_mode(BlendMode::Custom(swap_red_blue));
```

## Blend two images directly

The blend module can combine images without creating a canvas:

```rust
use abra::abra_core::{BlendMode, blend};
use abra::prelude::*;

let mut base = Image::read("assets/base.jpg")?;
let overlay = Image::read("assets/overlay.png")?;

blend::blend(&overlay).with_mode(BlendMode::Screen).apply(&mut base);
base.write("out/screened.png", None)?;
```

The source is placed at `(0, 0)` at full opacity with `BlendMode::Normal` unless you set otherwise. The destination always starts at its own origin; only the source is positioned, and only the destination pixels under the source are changed.

```rust
blend::blend(&overlay)
  .with_offset((80, 40))
  .with_opacity(0.6)
  .with_mode(BlendMode::SoftLight)
  .apply(&mut base);
```

The opacity is clamped to `0.0..=1.0` and is applied while compositing the blended result.

## Names and labels

Each mode has a machine-readable name and a display label. `from_name` turns a name back into a mode, and `BlendMode::ALL` lists every built-in mode in menu order:

```rust
assert_eq!(BlendMode::Multiply.name(), "multiply");
assert_eq!(BlendMode::Multiply.label(), "Multiply");
assert_eq!(BlendMode::from_name("color-burn"), Some(BlendMode::ColorBurn));

let menu: Vec<&str> = BlendMode::ALL.iter().map(|mode| mode.label()).collect();
```

Custom modes are named `"custom"`.

## Choosing a mode

- Use `Normal` for standard layer compositing.
- Use `Multiply`, `Darken`, or `LinearBurn` to deepen shadows.
- Use `Screen`, `Lighten`, or `LinearDodge` to add light.
- Use `Overlay`, `SoftLight`, or `HardLight` for contrast.
- Use `Difference` or `Exclusion` for visual comparisons.
- Use `Color`, `Hue`, `Saturation`, or `Luminosity` for component-based color work.
