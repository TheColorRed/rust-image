import { EditSection } from '../edit-sections';
import { ImageOperation } from '@alakazam/mobile';

export const transform: EditSection = {
  key: 'section-transform',
  label: 'Transform',
  previewThumbnails: true,
  controls: [
    {
      kind: 'action',
      key: 'action-rotate-left',
      label: 'Rotate Left',
      operation: ImageOperation.Rotate.new({ degrees: -90 }),
    },
    {
      kind: 'action',
      key: 'action-rotate-right',
      label: 'Rotate Right',
      operation: ImageOperation.Rotate.new({ degrees: 90 }),
    },
    {
      kind: 'action',
      key: 'action-flip-horizontal',
      label: 'Flip Horizontal',
      operation: ImageOperation.FlipHorizontal.new(),
    },
    {
      kind: 'action',
      key: 'action-flip-vertical',
      label: 'Flip Vertical',
      operation: ImageOperation.FlipVertical.new(),
    },
  ],
};
