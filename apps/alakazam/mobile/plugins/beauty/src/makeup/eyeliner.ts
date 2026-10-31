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
    key: toolKey('slider-eyeliner'),
    label: 'Eyeliner',
    person: true,
    face: true,
    needsModels: { title: 'Eyeliner', roles: MAKEUP_MODELS },
    display: 'tabulated',
    sliderLabel: ['Color', 'Width', 'Strength'],
    min: [0, 1.5, 0],
    max: [1, 3, 1],
    step: [0.01, 0.01, 0.01],
    defaultValue: [0, 1.5, 0.8],
    picker: ['hue', undefined, undefined],
    swatches: swatchesOf(faceColors, hue => [hue]),
    preload: makeupMasks,
    layer: {
      blend: BlendMode.Multiply,
      opacity: ([, , strength]) => strength,
      // Projects saved before the width slider have no third value; they get the thinnest width.
      paint: (sheet, [hue, width = 1.5]: number[], context) =>
        applyEyeMakeup('eyeliner', sheet, hue, 0, 0.07, width, context),
    },
  };
}
