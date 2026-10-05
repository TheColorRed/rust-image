import { EditSection } from '../edit-sections';
import { ImageOperation } from '@alakazam/mobile';

export const sharpen: EditSection = {
  key: 'section-sharpen',
  label: 'Sharpen',
  previewThumbnails: true,
  controls: [{ kind: 'action', key: 'action-sharpen', label: 'Sharpen', operation: ImageOperation.Sharpen.new() }],
};
