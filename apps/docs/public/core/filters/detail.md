---
title: Detail filters
order: 5
outline: deep
---

# Detail filters

Detail filters either increase local edge definition or reduce small variations. The family includes sharpening, general smoothing, and skin-aware smoothing. See [Apply options](../apply-options) for regional processing.

## Sharpen

`sharpen` increases local detail:

```rust
use abra::filters::prelude::sharpen::sharpen;

sharpen().apply(&mut image);
```

## Smooth

`smooth` reduces local variation across an image or selected area:

```rust
use abra::filters::prelude::smooth::smooth;

smooth().apply(&mut image);
```

## Skin smoothing

`smooth_skin` smooths skin while leaving the features on it sharp. It builds a mask in three steps and then smooths through it:

```rust
use abra::filters::prelude::smooth::smooth_skin;

smooth_skin(0.65).apply(&mut image);
```

1. **Skin color.** Each pixel is tested in YCbCr space, which keeps brightness apart from color, so skin in shadow and in sun both match. Whites, grays, dark hair and strongly red lips fall outside the range. Pixels near the range count partly, so the mask has no hard edge.
2. **Edges are excluded.** Where the brightness changes quickly from one pixel to the next, as at eyes, lashes, brows, lips, nostrils and strands of hair, the mask closes, and the area around the edge is protected too. Edges are measured as if the photo were about 1200 pixels across, by spacing the edge kernel's taps out on a larger photo, so a large photo is protected the same as a small one and its grain is not mistaken for edges.
3. **Edge-aware smoothing.** The masked area is smoothed with a [surface blur](./blur), which blurs flat areas and stays sharp at edges. Its radius is half a percent of the photo's long side, so a photo and a smaller copy of it look alike: the live preview and the full-size edit smooth to the same degree. The blur looks at no more than 10 pixels on each side of a pixel, so its cost does not grow with the photo. On a larger photo it spaces those pixels out instead (the surface blur's `with_step`), which reaches as far for the same cost. The widths of the two mask blurs scale with the photo too.

The amount is clamped to `0.0..=3.0`. Up to `1.0` it scales the mask, which is how much of the smoothing shows. Past `1.0` the mask is fully on and the smoothing itself gets stronger by that factor: the surface blur's radius and threshold are both multiplied by it (the threshold is held to 90). Around `2.0` is a strong, natural look, and `3.0` starts to look plastic. Existing masks can be combined with the detected skin mask through the shared application options.

The whole filter runs on the GPU when one is available, as a chain of shader passes (edge strength, widening the protected zone, skin color, feathering the mask, the surface blur, and the final blend), so the photo never leaves the GPU. The GPU result is within one level of the CPU result. On a 1920x1080 photo the GPU chain takes about 30 to 45 ms, against about 0.5 to 1.7 seconds on the CPU, which is the fallback when there is no GPU. Because the radius and the edge measurement come from the size of the whole photo, the filter always runs over the whole photo and is then limited to an area or mask, if one is given.

## Skin tone

`adjust_skin_tone` changes the brightness of existing skin tones without adding a tint or smoothing texture:

```rust
use abra::filters::prelude::skin::adjust_skin_tone;

adjust_skin_tone(-40).apply(&mut image); // Lighter.
adjust_skin_tone(40).apply(&mut image);  // Darker.
adjust_skin_tone(200).apply(&mut image); // Stronger darkening is allowed by the core.
```

Negative amounts lighten, positive amounts darken, and `0` leaves the image unchanged. `adjust_skin_tone` does not clamp
the amount to a control range. Values beyond `-100` and `100` produce stronger adjustments; each application chooses
and enforces its own input limits.

The filter uses the existing exposure effect's gamma curve to adjust midtones while keeping black and white fixed,
so lightening retains highlight detail. It shares skin detection and feathering with `tan_skin`.
Use `.with_mask(mask)` for a segmentation or person mask and `.with_feather(amount)` to control the boundary fade.
Both CPU processing and GPU previews reuse the existing exposure effect and preserve the photo's alpha.

## Choosing a detail filter

- Use `sharpen` to restore or emphasize local detail.
- Use `smooth` for general softening.
- Use `smooth_skin` when smoothing should be limited to detected skin tones and edges such as eyes and hair should stay sharp.
- Use `adjust_skin_tone` to lighten or darken existing skin tones while preserving texture.
- Use [Blur](./blur) for explicit blur kernels and focus effects.
