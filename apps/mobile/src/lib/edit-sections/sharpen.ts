import { EditSection } from "../edit-sections";

export const sharpen: EditSection = {
  key: 'section-sharpen',
  label: 'Sharpen',
  previewThumbnails: true,
  controls: [{ kind: 'action', key: 'action-sharpen', label: 'Sharpen', apply: image => image.sharpen() }],
};
