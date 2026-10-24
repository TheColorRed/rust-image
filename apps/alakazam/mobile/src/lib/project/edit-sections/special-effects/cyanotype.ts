import { cyanotype } from '@/src/effects/cyanotype';
import { SliderControl } from '@/src/lib/project/edit-sections';

export const cyanotypeControl = {
  kind: 'slider',
  key: 'action-cyanotype',
  label: 'Cyanotype',
  min: 0,
  max: 1,
  step: 0.01,
  defaultValue: 1,
  applyOnSelect: true,
  tool: (photo, [amount]: number[]) => cyanotype(photo, amount),
  thumb: photo => cyanotype(photo, 1),
} satisfies SliderControl<number>;
