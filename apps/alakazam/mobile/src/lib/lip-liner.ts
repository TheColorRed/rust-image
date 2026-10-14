import { Color } from '@abra/core/color';
import { gaussianBlur } from '@abra/core/filters';
import { Mask, type MaskLike } from '@abra/core/mask';
import { type ToolContext } from '@/src/lib/edit-sections';
import { derivedFor } from '@/src/lib/editor-preloads';
import { belongsTo, owning } from '@/src/lib/faces';
import { paintSheet, type Painting } from '@/src/lib/layers';

/** How wide the line is, as a fraction of the width of the mouth: the least (the Width slider's 0) and the most (its 1). */
const THINNEST = 0.008;
const THICKEST = 0.04;

/**
 * Draws a line of a hue (0 to 1 along `Gradient.hue()`) around the lips of the person in `context`, or of everyone when it names
 * none, on `sheet`, which becomes a layer over the photo, from thin at `width` 0 to thick at 1, as colorful as `saturation` and as
 * bright as `brightness` (both 0 to 1). The line sits on the edge of the lips, a little more outside than in. How much of it shows is
 * the layer's opacity and how it mixes with the lips is the layer's blend, so their light and shade show through.
 */
export const applyLipLiner = (
  sheet: Painting,
  hue: number,
  width: number,
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
      // The line only changes with the width, so it is made once for it and kept while the other sliders move.
      const line = derivedFor(preloaded, `lip-liner|${key}`, width.toFixed(3), () =>
        owning(own => {
          const bounds = (lips as MaskLike).bounds(0);
          if (!bounds) return undefined;
          const radius = Math.max(0.8, bounds.width * (THINNEST + (THICKEST - THINNEST) * width));
          // The lips blurred are half on at their edge. Doubled, they are full there and fade away outside it (the outer edge of
          // the line); taking away the part that is well inside leaves a band that straddles the edge.
          const spread = own((lips as MaskLike).toImage());
          own(gaussianBlur(radius)).apply(spread);
          const inside = own(spread.clone());
          spread.mul(2);
          inside.sub(127);
          inside.mul(4);
          return Mask.fromImage(spread).subtracted(own(Mask.fromImage(inside)));
        }),
      ) as MaskLike | undefined;
      if (line) regions.push(line);
    }
    paintSheet(sheet, regions, color);
  });
};
