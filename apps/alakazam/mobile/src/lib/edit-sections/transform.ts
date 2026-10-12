import { EditSection } from '../edit-sections';
import { FlipAxis, TransformFit, flip, rotate } from '@abra/core/transform';

export const transform: EditSection = {
  key: 'section-transform',
  label: 'Transform',
  previewThumbnails: true,
  controls: [
    {
      kind: 'action',
      key: 'action-rotate-left',
      label: 'Rotate Left',
      tool: photo => rotate(-90).withFit(TransformFit.Fill).apply(photo),
      thumb: photo => rotate(-90).withFit(TransformFit.Fill).apply(photo),
    },
    {
      kind: 'action',
      key: 'action-rotate-right',
      label: 'Rotate Right',
      tool: photo => rotate(90).withFit(TransformFit.Fill).apply(photo),
      thumb: photo => rotate(90).withFit(TransformFit.Fill).apply(photo),
    },
    {
      kind: 'action',
      key: 'action-flip-horizontal',
      label: 'Flip Horizontal',
      tool: photo => flip(FlipAxis.Horizontal).apply(photo),
      thumb: photo => flip(FlipAxis.Horizontal).apply(photo),
    },
    {
      kind: 'action',
      key: 'action-flip-vertical',
      label: 'Flip Vertical',
      tool: photo => flip(FlipAxis.Vertical).apply(photo),
      thumb: photo => flip(FlipAxis.Vertical).apply(photo),
    },
  ],
};
