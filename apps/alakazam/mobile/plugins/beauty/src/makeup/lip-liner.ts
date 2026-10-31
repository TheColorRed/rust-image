import { Color } from '@abra/core/color';
import { gaussianBlur } from '@abra/core/filters';
import { Mask, type MaskLike } from '@abra/core/mask';
import {
  belongsTo,
  toolKey,
  derivedFor,
  owning,
  paintSheet,
  type EditControl,
  type Painting,
  type ToolContext,
} from '@alakazam/sdk';
import { lipBlend, lipSwatches } from './lips';
import { MAKEUP_MODELS, makeupMasks } from './makeup-models';

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

/** The Lip Liner control. Its label is the name in the plugin's `plugin.yml`. */
export default function (): EditControl {
  return {
    kind: 'slider',
    key: toolKey('slider-lip-liner'),
    label: 'Lip Liner',
    person: true,
    face: true,
    needsModels: { title: 'Lip Liner', roles: MAKEUP_MODELS },
    display: 'tabulated',
    sliderLabel: ['Color', 'Width', 'Strength', 'Colorfulness', 'Brightness'],
    min: [0, 0, 0, 0, 0],
    max: [1, 1, 1, 1, 1],
    step: [0.01, 0.01, 0.01, 0.01, 0.01],
    // Opening the control draws a soft rosy-brown line at once, so there is something to change.
    defaultValue: [1 - 358 / 360, 0.4, 0.8, 0.8, 0.8],
    picker: ['hue', undefined, undefined, undefined, undefined],
    swatches: lipSwatches,
    // The AI finds the lips once, when the control opens; dragging only changes the line.
    preload: makeupMasks,
    layer: {
      blend: lipBlend,
      opacity: ([, , strength]) => strength,
      paint: (sheet, [hue, width, , saturation, brightness]: number[], context) =>
        applyLipLiner(sheet, hue, width, saturation, brightness, context),
    },
  };
}
