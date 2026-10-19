import { despeckle, median } from '@abra/core/filters';
import { EditSection, SliderControl } from "@/src/lib/project/edit-sections";

export const cleanup: EditSection = {
  key: 'section-cleanup',
  label: 'Cleanup',
  controls: [
    {
      kind: 'slider',
      key: 'action-median',
      label: 'Smooth Noise',
      min: 0,
      max: 8,
      step: 1,
      defaultValue: 0,
      applyOnSelect: false,
      deferred: true,
      tool: (photo, [value], { scale }) => median(value * scale).applyDeferred(photo),
    } satisfies SliderControl<number>,
    {
      kind: 'action',
      key: 'action-despeckle',
      label: 'Remove Dust',
      deferred: true,
      tool: photo => despeckle(1, 13).applyDeferred(photo),
    },
  ],
};
