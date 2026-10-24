import { bleachBypass } from '@/src/effects/bleach-bypass';
import { SliderControl } from '@/src/lib/project/edit-sections';

export const bleachBypassControl = {
  kind: 'slider',
  key: 'action-bleach-bypass',
  label: 'Bleach Bypass',
  min: 0,
  max: 1,
  step: 0.01,
  defaultValue: 0.6,
  applyOnSelect: true,
  tool: (photo, [amount]: number[]) => bleachBypass(photo, amount),
  thumb: photo => bleachBypass(photo, 1),
} satisfies SliderControl<number>;
