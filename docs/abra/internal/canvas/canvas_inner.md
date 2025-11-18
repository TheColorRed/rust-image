# CanvasInner (Internal)

Mutable implementation behind the public `Canvas` wrapper. Not exported.

## Responsibilities
- Track project identity (id, name), dimensions, and result buffer.
- Manage child canvases and layers (`Vec<Arc<Mutex<...>>>`).
- Orchestrate recomposition (`update_canvas`) and IO (`save`, `as_image`).
- Parent/child layout: anchor storage, rotation, origin, and global position.

## Recomposition Pipeline
1. Early exit if width/height are zero.
2. Create a blank RGBA canvas.
3. For each child canvas: apply anchor vs parent, call `update_canvas()` recursively.
4. Blend child canvases: optionally rotate child image, blend at `(x,y)` with opacity 1.0.
5. For each layer: `apply_pending_effects()`, apply anchor vs canvas dims, then blend with `blend_mode` and `opacity`.
6. Store final image as `result` and mark `needs_recompose = true`.

## Notes
- First added child/layer can define canvas size if initial size is 0×0.
- `origin` is stored for canvas; layers have their own origin.
- `anchor_to_canvas` stores anchor; actual placement occurs during composition.
