import { boxBlur, gaussianBlur } from '@abra/core/filters';
import { EditSection, SliderControl } from "@/src/lib/project/edit-sections";

export const blur: EditSection = {
  key: 'section-blur',
  label: 'Blur',
  controls: [
    {
      kind: 'slider',
      key: 'action-gaussian-blur',
      label: 'Blur',
      min: 0,
      max: 40,
      step: 1,
      defaultValue: 0,
      applyOnSelect: false,
      deferred: true,
      tool: (photo, [value], { scale }) => gaussianBlur(value * scale).applyDeferred(photo),
    } satisfies SliderControl<number>,
    {
      kind: 'slider',
      key: 'action-box-blur',
      label: 'Blur (Sharp Edges)',
      min: 0,
      max: 40,
      step: 1,
      defaultValue: 0,
      applyOnSelect: false,
      deferred: true,
      tool: (photo, [value], { scale }) => boxBlur(value * scale).applyDeferred(photo),
    } satisfies SliderControl<number>,
  ],
};
