import { EffectSpec } from "@alakazam/mobile";
import { EditSection } from "../edit-sections";

export const blur: EditSection = {
  key: 'section-blur',
  label: 'Blur',
  controls: [
    {
      kind: 'slider',
      key: 'action-gaussian-blur',
      label: 'Gaussian',
      min: 0,
      max: 40,
      step: 1,
      defaultValue: 0,
      applyOnSelect: false,
      apply: (image, value) => image.applyEffect(EffectSpec.GaussianBlur.new({ radius: value })),
      live: value => EffectSpec.GaussianBlur.new({ radius: value }),
    },
    {
      kind: 'slider',
      key: 'action-box-blur',
      label: 'Box',
      min: 0,
      max: 40,
      step: 1,
      defaultValue: 0,
      applyOnSelect: false,
      apply: (image, value) => image.boxBlur(value),
    },
  ],
};
