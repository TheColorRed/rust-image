import { BlendMode } from '@abra/core/blend';
import { MAKEUP_MODELS, makeupMasks, swatchesOf, type EditControl } from '@alakazam/host';
import { faceColors } from './face-colors';
import { applyEyeMakeup } from './eye-makeup';

export default function (): EditControl {
  return {
          kind: 'slider',
          key: 'slider-under-eye',
          label: 'Concealer',
          person: true,
          face: true,
          needsModels: { title: 'Concealer', roles: MAKEUP_MODELS },
          display: 'tabulated',
          sliderLabel: ['Color', 'Strength', 'Colorfulness', 'Brightness', 'Size'],
          min: [0, 0, 0, 0, 0.3],
          max: [1, 1, 1, 1, 3],
          step: [0.01, 0.01, 0.01, 0.01, 0.01],
          defaultValue: [1 - 24 / 360, 0.8, 0.8, 0.8, 1],
          picker: ['hue', undefined, undefined, undefined, undefined],
          swatches: swatchesOf(faceColors, (hue, saturation, brightness) => [hue, undefined, saturation, brightness]),
          preload: makeupMasks,
          layer: {
            blend: BlendMode.Color,
            opacity: ([, strength]) => strength,
            paint: (sheet, [hue, , saturation, brightness, fullness]: number[], context) =>
              applyEyeMakeup('under-eye', sheet, hue, saturation, brightness, fullness, context),
          },
        };
}
