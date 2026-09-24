---
title: Effects
outline: deep
order: 6
---

# Effects

`LayerEffects` queues visual effects for a layer or an entire canvas. Effects are applied during composition, so the original layer image remains available for later edits.

The current effect pipeline applies effects in this order:

1. Stroke
2. Drop shadow

## Layer effects

Use the builder returned by `layer.effects()`:

```rust
use abra::canvas::effects::{DropShadow, Stroke};
use abra::abra_core::{Color, Fill};

let logo = canvas.get_layer_by_name("Logo").unwrap();
logo.effects()
  .with_stroke(
    Stroke::new()
      .with_size(4)
      .with_fill(Fill::Solid(Color::white().into())),
  )
  .with_drop_shadow(
    DropShadow::new()
      .with_distance(8.0)
      .with_size(6.0),
  );
```

The builder is attached to the layer when it is dropped. Keep the chain in a statement or scope so the builder is dropped after all effects have been configured.

## Explicit effect configuration

Use `LayerEffects::new()` when building a value separately or clearing effects:

```rust
use abra::canvas::effects::LayerEffects;

let effects = LayerEffects::new();
logo.set_effects(effects);
```

You can also pass a configured builder to `set_effects`:

```rust
use abra::canvas::effects::{DropShadow, LayerEffects};

let effects = LayerEffects::new()
  .with_drop_shadow(DropShadow::new().with_distance(10.0));
logo.set_effects(effects);
```

## Stroke

A stroke draws an outline around the layer's content. Configure its size, fill, opacity, and position:

```rust
use abra::canvas::effects::Stroke;
use abra::abra_core::{Color, Fill};

let stroke = Stroke::new()
  .with_size(6)
  .with_fill(Fill::Solid(Color::white().into()))
  .with_opacity(0.8);

logo.effects().with_stroke(stroke);
```

The stroke is applied before a drop shadow when both are present.

## Drop shadow

A drop shadow can be configured with distance, angle, blur size, spread, opacity, fill, and blend mode:

```rust
use abra::canvas::effects::DropShadow;

let shadow = DropShadow::new()
  .with_distance(12.0)
  .with_angle(45.0)
  .with_size(8.0)
  .with_spread(0.2)
  .with_opacity(0.35);

logo.effects().with_drop_shadow(shadow);
```

Shadow padding can expand the rendered effect bounds. The canvas accounts for this offset during composition so the original content remains positioned correctly.

## Canvas effects

Apply effects to the entire canvas with `Canvas::set_effects`:

```rust
use abra::canvas::effects::{DropShadow, LayerEffects};

canvas.set_effects(
  LayerEffects::new()
    .with_drop_shadow(DropShadow::new().with_distance(8.0).with_size(6.0)),
);
```

Canvas effects run after the canvas's layers have been composed.

## Effect order

When both effects are configured, Abra applies them in this order:

```text
layer content -> stroke -> drop shadow -> canvas composition
```

Use a stroke when the outline should contribute to the shadow. Configure the stroke first through `with_stroke`; `LayerEffects` maintains the defined effect order during rendering.
