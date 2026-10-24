import { smooth } from '@abra/core/filters';
import { EditSection } from '@/src/lib/project/edit-sections';

export const detail: EditSection = {
  key: 'section-detail',
  label: 'Detail',
  previewThumbnails: true,
  controls: [
    {
      kind: 'action',
      key: 'action-smooth',
      label: 'Soften',
      deferred: true,
      tool: photo => smooth().applyDeferred(photo),
      thumb: photo => smooth().apply(photo),
    },
  ],
};
