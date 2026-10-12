import { sharpen as sharpenFilter } from '@abra/core/filters';
import { EditSection } from '../edit-sections';

export const sharpen: EditSection = {
  key: 'section-sharpen',
  label: 'Sharpen',
  previewThumbnails: true,
  controls: [
    {
      kind: 'action',
      key: 'action-sharpen',
      label: 'Sharpen',
      tool: photo => sharpenFilter().apply(photo),
      thumb: photo => sharpenFilter().apply(photo),
    },
  ],
};
