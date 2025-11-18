# layer_options_applier (Internal)

Applies `NewLayerOptions` to a freshly added layer.

## Behavior
- Anchor: default `Anchor::Center` if not provided.
- Size: apply via `layer_size_applier::apply_layer_size(layer, size, canvas_w, canvas_h)`.
- Opacity: set if provided.
- Blend mode: set if provided.

Called by `Canvas::add_layer_from_image` after pushing the layer to the canvas.
