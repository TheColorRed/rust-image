# CanvasTransform

Applies transforms to all layers on a canvas, keeping positions proportional where applicable.

## Methods
- `resize(width, height, algorithm)`: scales all layers proportionally to fit new canvas size.
- `resize_percentage(pct, algorithm)`: uniform scale by percentage.
- `resize_width(width, algorithm)`: scale along X; updates layer positions on X.
- `resize_height(height, algorithm)`: scale along Y; updates layer positions on Y.
- `resize_width_relative(dx, algorithm)`: grow/shrink by delta pixels on X; recenters horizontally.
- `resize_height_relative(dy, algorithm)`: grow/shrink by delta pixels on Y; recenters vertically.
- `crop(x, y, w, h)`: crops all layers to the intersection area, adjusting their positions.
- `rotate(deg, algorithm)`: rotates each layer image.

## Example
```rust
let mut t = canvas.transform();
t.resize(1200, 800, None)
 .crop(50, 50, 1100, 700)
 .rotate(3.0, None);
```
