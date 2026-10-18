import { sharpen as sharpenFilter } from '@abra/core/filters';
import { EditSection, SliderControl } from '../edit-sections';

/** The slider goes from 0 to 100; the filter's amount is a hundredth of it, so 25 is the filter's own default. */
export const sharpen: EditSection = {
  key: 'section-sharpen',
  label: 'Sharpen',
  previewThumbnails: true,
  controls: [
    {
      kind: 'slider',
      key: 'action-sharpen',
      label: 'Sharpen',
      min: 0,
      max: 100,
      step: 1,
      defaultValue: 0,
      applyOnSelect: false,
      deferred: true,
      tool: (photo, [value]) => sharpenFilter().withAmount(value / 100).applyDeferred(photo),
      thumb: photo => sharpenFilter().apply(photo),
    } satisfies SliderControl<number>,
  ],
};
