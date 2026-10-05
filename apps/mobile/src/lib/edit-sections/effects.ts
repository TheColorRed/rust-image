import { EffectSpec as Effect, ImageOperation } from '@alakazam/mobile';
import { EditSection } from '../edit-sections';

export const effects: EditSection = {
  key: 'section-effects',
  label: 'Effects',
  previewThumbnails: true,
  controls: [
    {
      kind: 'action',
      key: 'action-grayscale',
      label: 'Grayscale',
      live: () => Effect.Grayscale.new(),
    },
    {
      kind: 'action',
      key: 'action-invert',
      label: 'Invert',
      live: () => Effect.Invert.new(),
    },
    { kind: 'action', key: 'action-auto-tone', label: 'Auto Tone', operation: ImageOperation.AutoTone.new() },
    { kind: 'action', key: 'action-auto-color', label: 'Auto Color', operation: ImageOperation.AutoColor.new() },
    {
      kind: 'slider',
      key: 'action-threshold',
      label: 'Threshold',
      min: 0,
      max: 255,
      defaultValue: 128,
      apply: (image, value) => image.applyEffect(Effect.Threshold.new({ amount: value })),
      live: value => Effect.Threshold.new({ amount: value }),
    },
    {
      kind: 'action',
      key: 'action-posterize',
      label: 'Posterize',
      operation: ImageOperation.Posterize.new({ levels: 6 }),
    },
  ],
};
