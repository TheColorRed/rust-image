# Origin

Defines which point inside a layer or canvas is aligned to the Anchor.

Variants: same grid as `Anchor` plus `Custom(f32, f32)` where 0.0–1.0 are left/top to right/bottom.

Notes:
- Default is `Origin::Center`.
- Adjusts the reference point used during anchor placement.

```rust
use abra::canvas::Origin;
canvas.set_origin(Origin::TopLeft);
```
