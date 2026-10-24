import { EditSection } from '@/src/lib/project/edit-sections';
import { FlipAxis, flip, rotate } from '@abra/core/transform';
import { CROP_TOOL_KEY } from '@/src/lib/project/edit-sections/keys';

export const transform: EditSection = {
  key: 'section-transform',
  label: 'Transform',
  previewThumbnails: true,
  controls: [
    {
      kind: 'action',
      key: 'action-rotate-left',
      label: 'Rotate Left',
      repeatable: true,
      tool: photo => rotate(-90).apply(photo),
      thumb: photo => rotate(-90).apply(photo),
    },
    {
      kind: 'action',
      key: 'action-rotate-right',
      label: 'Rotate Right',
      repeatable: true,
      tool: photo => rotate(90).apply(photo),
      thumb: photo => rotate(90).apply(photo),
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
    // The crop changes the photo only when it is applied, so its thumbnail is the photo as it is.
    { kind: 'tool', key: CROP_TOOL_KEY, label: 'Crop', thumb: () => undefined },
  ],
};
