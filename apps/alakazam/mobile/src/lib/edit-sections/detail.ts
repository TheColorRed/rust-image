import { EditSection } from '../edit-sections';
import { ImageOperation } from '@alakazam/mobile';

export const detail: EditSection = {
  key: 'section-detail',
  label: 'Detail',
  previewThumbnails: true,
  controls: [{ kind: 'action', key: 'action-smooth', label: 'Smooth', operation: ImageOperation.Smooth.new() }],
};
