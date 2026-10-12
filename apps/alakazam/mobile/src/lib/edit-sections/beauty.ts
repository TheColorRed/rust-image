import { EffectSpec } from '@alakazam/mobile';
import { type EditSection } from '../edit-sections';
import { BLEMISH_TOOL_KEY, RED_EYE_TOOL_KEY, SKIN_SMOOTH_KEY, SKIN_TAN_KEY, SKIN_TONE_KEY } from './keys';

export const beauty: EditSection = {
  key: 'section-beauty',
  label: 'Beauty',
  controls: [
    {
      kind: 'slider',
      key: SKIN_SMOOTH_KEY,
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
      key: SKIN_TAN_KEY,
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
    {
      kind: 'slider',
      key: SKIN_TONE_KEY,
      label: 'Skin Tone',
      // Negative lightens the original skin; positive darkens it.
      min: -400,
      max: 400,
      step: 1,
      defaultValue: 0,
      applyOnSelect: false,
      live: value => EffectSpec.SkinTone.new({ amount: value }),
    },
    { kind: 'tool', key: RED_EYE_TOOL_KEY, label: 'Red Eye' },
    { kind: 'tool', key: BLEMISH_TOOL_KEY, label: 'Blemish' },
  ],
};
