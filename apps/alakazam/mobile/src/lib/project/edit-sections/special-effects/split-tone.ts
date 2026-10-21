import { splitTone } from '@/src/effects/split-tone';
import { SliderControl } from '@/src/lib/project/edit-sections';

export const splitToneControl = {
  kind: 'slider',
  key: 'action-split-tone',
  label: 'Split Tone',
  min: 0,
  max: 1,
  step: 0.01,
  defaultValue: 0.6,
  applyOnSelect: true,
  tool: (photo, [amount]: number[]) => splitTone(photo, amount),
  thumb: photo => splitTone(photo, 1),
} satisfies SliderControl<number>;
