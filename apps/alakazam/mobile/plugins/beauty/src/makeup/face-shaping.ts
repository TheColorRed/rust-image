import { BlendMode } from '@abra/core/blend';
import { toolKey, type EditControl } from '@alakazam/plugin';
import { MAKEUP_MODELS, makeupMasks } from './makeup-models';
import { applySculpt } from './sculpt';

export default function (): EditControl {
  return {
          kind: 'slider',
          key: toolKey('slider-contour'),
          label: 'Face Shaping',
          person: true,
          face: true,
          needsModels: { title: 'Face Shaping', roles: MAKEUP_MODELS },
          // First the shade of the shadow (a bar from a light tan to a dark brown), then how much, how soft it is, how big and how much the hollow fans out towards the mouth.
          display: 'tabulated',
          sliderLabel: ['Shade', 'Size', 'Spread', 'Strength', 'Softness'],
          min: [0, 0.5, 0, 0, 0],
          max: [1, 1.5, 1, 1, 1],
          step: [0.01, 0.01, 0.01, 0.01, 0.01],
          // Opening the control adds a soft shade under the cheekbones and beside the nose at once.
          defaultValue: [0.5, 1.5, 0, 0.2, 1],
          picker: ['shade', undefined, undefined, undefined, undefined],
          // The AI finds the face once, when the control opens; dragging only changes how it looks.
          preload: makeupMasks,
          layer: {
            blend: BlendMode.Multiply,
            opacity: ([, , , strength]) => strength,
            paint: (sheet, [shade, size, fan = 0, , softness]: number[], context) =>
              applySculpt('contour', sheet, softness, size, shade, context, fan),
          },
        };
}
