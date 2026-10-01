import { EffectSpec as Effect } from "@alakazam/mobile";
import { EditSection } from "../edit-sections";

export const effects: EditSection = {
  key: 'section-effects',
  label: 'Effects',
  previewThumbnails: true,
  controls: [
    {
      kind: 'action', key: 'action-grayscale', label: 'Grayscale',
      apply: image => image.applyEffect(Effect.Grayscale.new()),
      live: () => Effect.Grayscale.new()
    },
    {
      kind: 'action', key: 'action-invert', label: 'Invert',
      apply: image => image.applyEffect(Effect.Invert.new()),
      live: () => Effect.Invert.new()
    },
    { kind: 'action', key: 'action-auto-tone', label: 'Auto Tone', apply: image => image.autoTone() },
    { kind: 'action', key: 'action-auto-color', label: 'Auto Color', apply: image => image.autoColor() },
    {
      kind: 'slider', key: 'action-threshold', label: 'Threshold',
      min: 0, max: 255, defaultValue: 128,
      apply: (image, value) => image.applyEffect(Effect.Threshold.new({ amount: value })),
      live: value => Effect.Threshold.new({ amount: value })
    },
    { kind: 'action', key: 'action-posterize', label: 'Posterize', apply: image => image.posterize(6) },
  ],
};
