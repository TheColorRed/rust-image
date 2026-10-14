import { Color } from '@abra/core/color';
import { gaussianBlur } from '@abra/core/filters';
import { Mask, type MaskLike } from '@abra/core/mask';
import { derivedFor } from '@/src/lib/editor-preloads';
import { belongsTo, owning } from '@/src/lib/faces';
import { paintSheet, type Painting } from '@/src/lib/layers';
import { type ToolContext } from '@/src/lib/edit-sections';

/**
 * How wide the softest edge is, as a fraction of the width of the mouth: the Softness slider's 1. Lipstick has a fairly hard
 * edge, so the slider starts low; 0.04 of the mouth's width was too fuzzy.
 */
const MOST_SOFT_EDGE = 0.04;
/**
 * Paints lipstick of a hue (0 to 1 along `Gradient.hue()`, which the bar of hues shows: hue 360 at 0, falling to 0 at 1) on the lips
 * of the person in `context`, or of everyone when it names none, on `sheet`, which becomes a layer over the photo, as colorful as
 * `saturation` and as bright as `brightness` (both 0 to 1; black is 0 brightness, white is 0 saturation and 1 brightness), with an
 * edge from hard at `softness` 0 to soft at 1. How much of it shows is the layer's opacity and how it mixes with the lips is the
 * layer's blend, so the lips' own light and shade show through.
 */
export const applyLipstick = (
  sheet: Painting,
  hue: number,
  softness: number,
  saturation: number,
  brightness: number,
  { preloaded, person, others }: ToolContext,
) => {
  if (!preloaded || Array.isArray(preloaded)) return;
  owning(keep => {
    const color = keep(Color.fromHsv(360 * (1 - hue), saturation, brightness));
    const regions: MaskLike[] = [];
    for (const [key, lips] of Object.entries(preloaded)) {
      const [kind, box] = key.split('|');
      if (kind !== 'lipstick' || !box) continue;
      if (!belongsTo(box, sheet, person, others)) continue;
      // The edge only changes with the softness, so it is made once for it and kept while the other sliders move.
      const softened = derivedFor(preloaded, key, softness.toFixed(3), () =>
        owning(own => {
          const radius = (lips as MaskLike).bounds(0)!.width * MOST_SOFT_EDGE * softness;
          if (radius < 0.5) return undefined;
          const blurred = own((lips as MaskLike).toImage());
          own(gaussianBlur(radius)).apply(blurred);
          return Mask.fromImage(blurred);
        }),
      ) as MaskLike | undefined;
      const edge = softened ?? (lips as MaskLike);
      regions.push(edge);
    }
    paintSheet(sheet, regions, color);
  });
};
