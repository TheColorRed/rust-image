import { thermal } from '@/src/effects/thermal';
import { SliderControl } from '@/src/lib/project/edit-sections';

export const thermalControl = {
  kind: 'slider',
  key: 'action-thermal',
  label: 'Thermal',
  min: 0,
  max: 1,
  step: 0.01,
  defaultValue: 1,
  applyOnSelect: true,
  tool: (photo, [amount]: number[]) => thermal(photo, amount),
  thumb: photo => thermal(photo, 1),
} satisfies SliderControl<number>;
