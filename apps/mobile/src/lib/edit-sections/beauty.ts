import { EffectSpec } from "@alakazam/mobile";
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
      // Up to 1 is how much of the smoothing shows; past 1 the smoothing itself gets stronger, up to 3.
      max: 3,
      step: 0.1,
      defaultValue: 0,
      applyOnSelect: false,
      live: value => EffectSpec.SkinSmooth.new({ amount: value }),
    },
    {
      kind: 'slider',
      key: 'action-skin-tan',
      label: 'Skin Tan',
      // Where on the tan scale: none at 0, the darkest at 1.
      min: 0,
      max: 1,
      step: 0.01,
      defaultValue: 0,
      applyOnSelect: false,
      picker: 'skin-tan',
      live: value => EffectSpec.SkinTan.new({ offset: value }),
    },
    { kind: 'tool', key: BLEMISH_TOOL_KEY, label: 'Blemish' },
  ],
};
