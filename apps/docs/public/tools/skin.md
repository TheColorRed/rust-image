---
title: Skin tools
order: 4
outline: deep
---

# Skin tools

Skin tools generate a reusable skin mask, then compose it with the relevant general-purpose operation: Surface Blur for smoothing, color multiplication for tanning, and Exposure for tone. They are available from `abra::tools::prelude`.

```rust
use abra::tools::prelude::*;

skin_smooth(0.65).apply(&mut image);
skin_tan(Color::tan()).apply(&mut image);
skin_tone(-40).apply(&mut image); // Lighter.
skin_tone(40).apply(&mut image);  // Darker.
```

## Detection and masks

The shared detector identifies skin-colored pixels in YCbCr space, protects image edges such as eyes, brows, hair, and lips, then feathers the boundary. Each tool detects that mask from the unedited source image. Pass a segmentation mask when one is available:

```rust
let tool = skin_tone(-40)
  .with_mask(mask)
  .with_feather(3.0);

tool.apply(&mut image);
```

`with_mask` replaces color-based detection. The supplied mask is resized to the photo and softened with `with_feather`.

## Smoothing

`skin_smooth` runs Surface Blur through the skin mask. Amounts from `0.0` to `1.0` control how much of the result shows. Higher values strengthen the blur itself, up to `3.0`.

## Tan

`skin_tan` multiplies the skin color by the supplied color. White leaves the image unchanged; the color alpha controls the tan strength.

## Tone

`skin_tone` uses Exposure's gamma curve, so it changes midtones without adding a tint. Negative amounts lighten, positive amounts darken, and `0` is unchanged. The core does not limit this amount; each control can choose its own range.
