import { despeckle, median } from '@abra/core/filters';
import { EditSection, SliderControl } from "../edit-sections";

export const cleanup: EditSection = {
  key: 'section-cleanup',
  label: 'Cleanup',
  controls: [
    {
      kind: 'slider',
      key: 'action-median',
      label: 'Median',
      min: 0,
      max: 8,
      step: 1,
      defaultValue: 0,
      applyOnSelect: false,
      tool: (photo, [value], { scale }) => median(value * scale).apply(photo),
    } satisfies SliderControl<number>,
    {
      kind: 'action',
      key: 'action-despeckle',
      label: 'Despeckle',
      tool: photo => despeckle(1, 13).apply(photo),
    },
  ],
};
