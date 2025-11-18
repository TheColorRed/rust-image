# LayerInner (Internal)

Mutable implementation behind the public `Layer` wrapper. Not exported.

## Responsibilities
- Track image, visibility, opacity, blend mode, position, and identity (id).
- Maintain references to owning canvas (`Arc<Mutex<CanvasInner>>`).
- Anchor/origin bookkeeping, including `anchor_dimensions` and `anchor_offset`.
- Effect pipeline management (`LayerEffects`).
- z-order manipulation (`set_index`, move up/down/top/bottom).

## Anchoring
- `anchor_to_canvas(Anchor)`: store anchor; applied during render.
- `set_origin(Origin)`: change in-layer reference point for anchor math.
- `apply_anchor_with_canvas_dimensions(w,h)`: compute `(x,y)` from parent dims and `anchor_dimensions`.
- `anchor_dimensions`: override size for placement (e.g., after effects growth).
- `anchor_offset`: adjust final placement so effects don’t shift visuals.

## Effects
- `apply_pending_effects()` clones/apply via `LayerEffects`, updates `image`.
- Stroke is applied before DropShadow.

## Copy-on-Write Image
- `image_mut()` uses `Arc::make_mut`.
