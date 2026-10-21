import { chiaroscuro } from '@/src/effects/chiaroscuro';
import { SliderControl } from '@/src/lib/project/edit-sections';

export const chiaroscuroControl = {
  kind: 'slider',
  key: 'action-chiaroscuro',
  label: 'Chiaroscuro',
  sliderLabel: ['Strength', 'Color'],
  min: [0, 0],
  max: [1, 1],
  step: [0.001, 0.001],
  defaultValue: [0.5, 1],
  swatches: [
    { label: 'Color', color: '#d9a88a', values: [undefined, 1] },
    { label: 'Black and white', color: '#7a7a7a', values: [undefined, 0] },
  ],
  applyOnSelect: true,
  tool: (photo, [amount, color]) => chiaroscuro(photo, amount, color),
  thumb: photo => chiaroscuro(photo, 1),
} satisfies SliderControl<number>;
