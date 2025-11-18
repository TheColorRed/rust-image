# NewLayerOptions and LayerSize

Controls how a new layer is configured when added to a `Canvas`.

## LayerSize
- `Maintain`: keep original size.
- `Contain(algorithm)`: scale to fit within canvas.
- `Cover(algorithm)`: scale to cover canvas, may crop.
- `Specific(w, h, algorithm)`: exact target size.
- `Percentage(pct, algorithm)`: scale by percentage.

## NewLayerOptions
Builder-style API:
- `with_size(LayerSize)`
- `with_anchor(Anchor)`
- `with_opacity(f32)` (0.0–1.0)
- `with_blend_mode(fn(RGBA, RGBA) -> RGBA)`

## Example
```rust
use abra::{Canvas, Anchor};
use abra::canvas::{NewLayerOptions, LayerSize};

let opts = NewLayerOptions::new()
  .with_size(LayerSize::Contain(None))
  .with_anchor(Anchor::BottomRight)
  .with_opacity(0.85);

let canvas = Canvas::new("Config").add_layer_from_path("Pic", "assets/pic.png", Some(opts));
```
