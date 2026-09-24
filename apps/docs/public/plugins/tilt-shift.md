---
title: Tilt-shift
order: 3
outline: deep
---

# Tilt-shift

The `TiltShift` plugin is intended to make a scene resemble a miniature by keeping a focus region sharp while blurring areas outside it, with increased saturation and contrast.

## Plugin metadata

```rust
use abra::plugin::Plugin;
use tilt_shift::TiltShift;

let plugin = TiltShift;
assert_eq!(plugin.name(), "Tilt Shift");
println!("{}", plugin.description());
```

The plugin reports this description:

> Applies a tilt-shift effect to the image, simulating a miniature scene by blurring areas outside a defined focus region and increasing saturation and contrast.

## Current implementation status

The current `TiltShift::apply` implementation creates an empty `PluginResult` and returns it successfully. It does not yet accept an input canvas or image, define a focus region, blur pixels, or add an output to the result.

```rust
use abra::plugin::Plugin;
use tilt_shift::TiltShift;

let mut plugin = TiltShift;
let result = plugin.apply().unwrap();
assert!(result.is_empty());
```

Treat this crate as an API placeholder until the processing pipeline and input parameters are implemented.

## Intended workflow

A completed tilt-shift plugin will need an input image or canvas, a focus-region definition, and effect settings such as blur strength and saturation. The expected high-level workflow is:

1. Load or receive the source image.
2. Define the sharp focus band or region.
3. Blur pixels outside the focus region.
4. Increase saturation and contrast.
5. Return the processed image or canvas in `PluginResult`.

Those parameters are not part of the current public `TiltShift` API.
