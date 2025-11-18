# AddCanvasOptions

Options used when adding a child canvas to a parent.

Fields (builder-style):
- `with_anchor(Anchor)`: anchor relative to parent (default `Center`).
- `with_position(x, y)`: explicit top-left position; overrides anchor if provided.
- `with_rotation(degrees)`: child rotation in degrees.

```rust
use abra::canvas::AddCanvasOptions;
parent.add_canvas(child, Some(AddCanvasOptions::new().with_anchor(abra::Anchor::TopRight).with_position(-20, 10)));
```
