import { sharpen as sharpenFilter } from '@abra/core/filters';
import { owning } from '@/src/lib/native/native-handles';
import { EditSection, SliderControl } from '@/src/lib/project/edit-sections';

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
      tool: (photo, [value]) => owning(keep => keep(keep(sharpenFilter()).withAmount(value / 100)).applyDeferred(photo)),
      thumb: photo => owning(keep => keep(sharpenFilter()).apply(photo)),
    } satisfies SliderControl<number>,
  ],
};
