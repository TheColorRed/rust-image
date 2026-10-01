import { EditSection } from "../edit-sections";

/** Key of the blemish tool control; the editor dispatches on it to render the reticle and controls. */
export const BLEMISH_TOOL_KEY = 'action-blemish';

export const beauty: EditSection = {
  key: 'section-beauty',
  label: 'Beauty',
  controls: [
    {
      kind: 'slider',
      key: 'action-skin-smooth',
      label: 'Skin Smooth',
      min: 0,
      max: 1,
      step: 0.1,
      defaultValue: 0,
      applyOnSelect: false,
      apply: (image, value) => image.smoothSkin(value),
    },
    { kind: 'tool', key: BLEMISH_TOOL_KEY, label: 'Blemish' },
  ],
};
