# LayerEffects

Queues visual effects to be applied during layer rendering.

- Effects apply in order: Stroke → DropShadow.
- Use `layer.effects()` to scope effects to a layer (auto-commit on drop), or build and pass via `Layer::set_effects`.

## Example
```rust
use abra::canvas::effects::{LayerEffects, Stroke, DropShadow};
use abra::color::{Color, Fill};

let logo = canvas.get_layer_by_name("Logo").unwrap();
logo.effects()
  .with_stroke(Stroke::new().with_size(4).with_fill(Fill::Solid(Color::white())))
  .with_drop_shadow(DropShadow::new().with_distance(8.0).with_size(6.0));
```
