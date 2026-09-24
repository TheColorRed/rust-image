---
title: Child canvas options
order: 4
outline: deep
---

# Child canvas options

`AddCanvasOptions` controls how a child canvas is positioned when it is added to a parent canvas.

## Configure placement

```rust
use abra::canvas::{AddCanvasOptions, Anchor, Canvas};

let parent = Canvas::new_blank("Parent", 1200, 800);
let child = Canvas::new_blank("Child", 400, 300);
let options = AddCanvasOptions::new()
  .with_anchor(Anchor::BottomRight)
  .with_position(-24, -24)
  .with_rotation(5.0);

parent.add_canvas(child, Some(options));
```

The exact `add_canvas` overload may accept the child name or canvas reference used by your integration; the options value is the same.

## Defaults

`AddCanvasOptions::new()` defaults to:

- `Anchor::Center`
- no position offset
- no rotation

`with_position` adds a pixel offset after anchor placement. `with_rotation` stores a rotation in degrees for the child canvas.

## API summary

| API                       | Purpose                                        |
| ------------------------- | ---------------------------------------------- |
| `AddCanvasOptions::new()` | Create default child-canvas placement options. |
| `with_anchor(anchor)`     | Anchor the child within its parent.            |
| `with_position(x, y)`     | Add a pixel position offset.                   |
| `with_rotation(degrees)`  | Rotate the child canvas during composition.    |
