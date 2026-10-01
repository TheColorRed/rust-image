import { EditSection } from "../edit-sections";

export const transform: EditSection = {
  key: 'section-transform',
  label: 'Transform',
  previewThumbnails: true,
  controls: [
    { kind: 'action', key: 'action-rotate-left', label: 'Rotate Left', apply: image => image.rotate(-90) },
    { kind: 'action', key: 'action-rotate-right', label: 'Rotate Right', apply: image => image.rotate(90) },
    {
      kind: 'action',
      key: 'action-flip-horizontal',
      label: 'Flip Horizontal',
      apply: image => image.flipHorizontal(),
    },
    { kind: 'action', key: 'action-flip-vertical', label: 'Flip Vertical', apply: image => image.flipVertical() },
  ],
};
