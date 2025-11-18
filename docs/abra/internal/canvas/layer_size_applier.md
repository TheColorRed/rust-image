# layer_size_applier (Internal)

Resizes a layer according to `LayerSize` strategy.

## Strategies
- `Maintain`: no-op.
- `Contain(alg)`: scale to fit within `(canvas_w, canvas_h)`.
- `Cover(alg)`: scale to cover `(canvas_w, canvas_h)`; may crop.
- `Specific(w,h,alg)`: absolute size.
- `Percentage(pct,alg)`: scale by percentage.

Uses `Image` resize traits implemented under `crate::transform`.
