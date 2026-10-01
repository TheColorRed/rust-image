import { EditSection } from "../edit-sections";

export const detail: EditSection = {
  key: 'section-detail',
  label: 'Detail',
  previewThumbnails: true,
  controls: [{ kind: 'action', key: 'action-smooth', label: 'Smooth', apply: image => image.smooth() }],
};
