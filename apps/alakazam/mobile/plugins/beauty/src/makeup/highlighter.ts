import { BlendMode } from '@abra/core/blend';
import {
  toolKey,
  type EditControl,
} from '@alakazam/sdk';
import { applySculpt } from './sculpt';
import { MAKEUP_MODELS, makeupMasks } from './makeup-models';

export default function (): EditControl {
  return {
    kind: 'slider',
    key: toolKey('slider-highlighter'),
    label: 'Highlighter',
    person: true,
    face: true,
    needsModels: { title: 'Highlighter', roles: MAKEUP_MODELS },
    display: 'tabulated',
    sliderLabel: ['Color', 'Size', 'Spread', 'Strength', 'Softness'],
    min: [0, 0.5, 0, 0, 0],
    max: [1, 1.5, 1, 1, 1],
    step: [0.01, 0.01, 0.01, 0.01, 0.01],
    // Opening the control adds a light glow to the cheekbones, nose, forehead and chin at once. Fan swings each cheek's glow in
    // towards the nose.
    defaultValue: [0, 1, 0, 0.2, 1],
    picker: ['glow', undefined, undefined, undefined, undefined],
    // Tints people ask for by name, each a place on the bar (the bar has a stop for each, evenly spaced).
    swatches: [
      { label: 'Pearl', color: 'rgb(255,250,245)', values: [0, undefined, undefined, undefined, undefined] },
      {
        label: 'Champagne',
        color: 'rgb(247,231,206)',
        values: [0.25, undefined, undefined, undefined, undefined],
      },
      { label: 'Gold', color: 'rgb(234,200,120)', values: [0.5, undefined, undefined, undefined, undefined] },
      { label: 'Peach', color: 'rgb(255,190,150)', values: [0.75, undefined, undefined, undefined, undefined] },
      { label: 'Rose', color: 'rgb(244,170,180)', values: [1, undefined, undefined, undefined, undefined] },
    ],
    // The AI finds the face once, when the control opens; dragging only changes how it looks.
    preload: makeupMasks,
    layer: {
      blend: BlendMode.Screen,
      opacity: ([, , , strength]) => strength,
      paint: (sheet, [tint, size, fan = 0, , softness]: number[], context) =>
        applySculpt('highlighter', sheet, softness, size, tint, context, fan),
    },
  };
}
