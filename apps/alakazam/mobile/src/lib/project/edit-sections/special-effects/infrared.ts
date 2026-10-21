import { infrared } from '@/src/effects/infrared';
import { SliderControl } from '@/src/lib/project/edit-sections';

export const infraredControl = {
  kind: 'slider',
  key: 'action-infrared',
  label: 'Infrared',
  min: 0,
  max: 1,
  step: 0.01,
  defaultValue: 0,
  picker: 'infrared',
  tool: (photo, [value]) => infrared(photo, value),
  thumb: photo => infrared(photo, 0),
} satisfies SliderControl<number>;
