import { BlendMode } from '@abra/core/blend';
import { swatchesOf, toolKey, type EditControl } from '@alakazam/sdk';
import { MAKEUP_MODELS, makeupMasks } from './makeup-models';
import { faceColors } from './face-colors';
import { applyBlush } from './blush';

export default function (): EditControl {
  return {
          kind: 'slider',
          key: toolKey('slider-blush'),
          label: 'Blush',
          person: true,
          face: true,
          needsModels: { title: 'Blush', roles: MAKEUP_MODELS },
          // First the color, then how much, how soft it is and how wide, and for a custom color how vivid and bright.
          display: 'tabulated',
          sliderLabel: ['Color', 'Size', 'Strength', 'Softness', 'Colorfulness', 'Brightness'],
          min: [0, 0.5, 0, 0, 0, 0],
          max: [1, 3, 1, 1, 1, 1],
          step: [0.01, 0.01, 0.01, 0.01, 0.01, 0.01],
          // Opening the control puts a soft pink on the cheeks at once, so there is something to change.
          defaultValue: [1 - 350 / 360, 2, 0.4, 1, 0.8, 0.8],
          picker: ['hue', undefined, undefined, undefined, undefined, undefined],
          swatches: swatchesOf(faceColors, (hue, saturation, brightness) => [
            hue,
            undefined,
            undefined,
            undefined,
            saturation,
            brightness,
          ]),
          // The AI finds the face once, when the control opens; dragging only changes how the blush looks.
          preload: makeupMasks,
          layer: {
            blend: BlendMode.Color,
            opacity: ([, , strength]) => strength,
            paint: (sheet, [hue, size, , softness, saturation, brightness]: number[], context) =>
              applyBlush(sheet, hue, softness, size, saturation, brightness, context),
          },
        };
}
