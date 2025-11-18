# Stroke

Draws an outline around the layer’s bounds.

Fields:
- `fill: Fill`
- `opacity: f32`
- `size: u32`
- `position: OutlinePosition` (`Inside` | `Outside` | `Center`)

Builder: `with_size`, `with_fill`, `with_opacity`.

Notes:
- Opacity is applied to stroke alpha; original alpha is preserved to avoid fringe artifacts.
