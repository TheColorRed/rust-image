import { smooth } from '@abra/core/filters';
import { EditSection } from '../edit-sections';

export const detail: EditSection = {
  key: 'section-detail',
  label: 'Detail',
  previewThumbnails: true,
  controls: [
    {
      kind: 'action',
      key: 'action-smooth',
      label: 'Smooth',
      deferred: true,
      tool: photo => smooth().applyDeferred(photo),
      thumb: photo => smooth().apply(photo),
    },
  ],
};
