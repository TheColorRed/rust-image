import { boxBlur, gaussianBlur } from '@abra/core/filters';
import { EditSection, SliderControl } from "../edit-sections";

export const blur: EditSection = {
  key: 'section-blur',
  label: 'Blur',
  controls: [
    {
      kind: 'slider',
      key: 'action-gaussian-blur',
      label: 'Gaussian',
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
      label: 'Box',
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
