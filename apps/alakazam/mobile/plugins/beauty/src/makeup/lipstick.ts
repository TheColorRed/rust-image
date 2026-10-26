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
} from '@alakazam/plugin';
import { lipBlend, lipSwatches } from './lips';
import { MAKEUP_MODELS, makeupMasks } from './makeup-models';

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
          // The lips at a small size of the photo (a turned screen) can leave nothing in the mask, and then there is no edge to soften.
          const bounds = (lips as MaskLike).bounds(0);
          if (!bounds) return undefined;
          const radius = bounds.width * MOST_SOFT_EDGE * softness;
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

/** The Lipstick control. Its label is the name in the plugin's `plugin.yml`. */
export default function (): EditControl {
  return {
    kind: 'slider',
    key: toolKey('slider-lipstick'),
    label: 'Lipstick',
    person: true,
    face: true,
    needsModels: { title: 'Lipstick', roles: MAKEUP_MODELS },
    // First the color (a swatch or the bar), then how much of it, how soft the edge is, and for a custom color how vivid and bright.
    display: 'tabulated',
    sliderLabel: ['Color', 'Strength', 'Softness', 'Colorfulness', 'Brightness'],
    min: [0, 0, 0, 0, 0],
    max: [1, 1, 1, 1, 1],
    step: [0.01, 0.01, 0.01, 0.01, 0.01],
    // Opening the control puts this lipstick on at once (red, lightly), so there is something to change.
    defaultValue: [0, 0.8, 1, 0.8, 0.8],
    picker: ['hue', undefined, undefined, undefined, undefined],
    // Colors people ask for by name, from a shade chart of real lipsticks (each color is read from the chart; its name is the
    // plain description of it). Shades that were nearly the same color (a color distance under about 8) are left out, so every
    // swatch is a choice a person can see. A swatch is a hue on the bar (1 minus its degrees around the wheel over 360, since the bar runs
    // backwards), how vivid it is and how bright. A "sheer" shade also sets a light strength. Black and white come first.
    swatches: lipSwatches,
    // The AI finds the lips once, when the control opens; dragging only changes the color, the strength or the edge.
    preload: makeupMasks,
    layer: {
      blend: lipBlend,
      opacity: ([, amount]) => amount,
      paint: (sheet, [hue, , softness, saturation, brightness]: number[], context) =>
        applyLipstick(sheet, hue, softness, saturation, brightness, context),
    },
  };
}
