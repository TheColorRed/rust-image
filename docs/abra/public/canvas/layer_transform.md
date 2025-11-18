# LayerTransform

Thin proxy that delegates all transforms to the underlying image.

## Methods
- `resize(width, height, algorithm)`
- `resize_percentage(pct, algorithm)`
- `resize_width(width, algorithm)` / `resize_height(height, algorithm)`
- `resize_width_relative(dx, algorithm)` / `resize_height_relative(dy, algorithm)`
- `crop(x, y, w, h)`

## Example
```rust
let logo = canvas.get_layer_by_name("Logo").unwrap();
logo.transform()
    .resize_percentage(0.5, None)
    .crop(0, 0, 256, 256);
```
