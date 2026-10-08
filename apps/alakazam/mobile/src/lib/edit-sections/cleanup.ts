import { EditSection } from "../edit-sections";

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
      apply: (image, value) => image.median(value),
    },
    { kind: 'action', key: 'action-despeckle', label: 'Despeckle', apply: image => image.despeckle() },
  ],
};
