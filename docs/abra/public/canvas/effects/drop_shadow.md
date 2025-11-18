# DropShadow

Configures a blurred, offset shadow behind a layer.

Fields:
- `fill: Fill` (default black)
- `blend_mode: fn(RGBA, RGBA) -> RGBA` (default normal)
- `opacity: f32` (default 0.35)
- `angle: f32`, `distance: f32`
- `spread: f32` (0.0–1.0)
- `size: f32` (blur radius)

Builder:
- `with_distance`, `with_angle`, `with_size`, `with_spread`, `with_fill`, `with_opacity`, `with_blend_mode`

Notes:
- Shadow canvas expands to accommodate offset and blur; original is composited atop.
