import { BlendMode } from '@abra/core/blend';
import {
  swatchesOf,
  toolKey,
  type EditControl,
} from '@alakazam/sdk';
import { applyEyeMakeup } from './eye-makeup';
import { faceColors } from './face-colors';
import { MAKEUP_MODELS, makeupMasks } from './makeup-models';

export default function (): EditControl {
  return {
    kind: 'slider',
    key: toolKey('slider-brow-bone'),
    label: 'Brow Highlight',
    person: true,
    face: true,
    needsModels: { title: 'Brow Highlight', roles: MAKEUP_MODELS },
    display: 'tabulated',
    sliderLabel: ['Color', 'Size', 'Strength', 'Colorfulness', 'Brightness'],
    min: [0, 0.3, 0, 0, 0],
    max: [1, 3, 1, 1, 1],
    step: [0.01, 0.01, 0.01, 0.01, 0.01],
    defaultValue: [1 - 38 / 360, 1, 0.8, 0.8, 0.8],
    picker: ['hue', undefined, undefined, undefined, undefined],
    swatches: swatchesOf(faceColors, (hue, saturation, brightness) => [hue, undefined, saturation, brightness]),
    preload: makeupMasks,
    layer: {
      blend: BlendMode.SoftLight,
      opacity: ([, , strength]) => strength,
      paint: (sheet, [hue, fullness, , saturation, brightness]: number[], context) =>
        applyEyeMakeup('brow-bone', sheet, hue, saturation, brightness, fullness, context),
    },
  };
}
