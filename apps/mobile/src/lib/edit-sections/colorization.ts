import { EffectSpec, PhotoFilterPreset } from "@alakazam/mobile";
import { EditSection } from "../edit-sections";

export const colorization: EditSection = {
  key: 'section-colorization',
  label: 'Colorization',
  previewThumbnails: true,
  controls: [
    {
      kind: 'action',
      key: 'action-colorize-warm',
      label: 'Warm',
      group: 'colorize',
      apply: image => image.photoFilter(PhotoFilterPreset.Warming, 0.35),
    },
    {
      kind: 'action',
      key: 'action-colorize-cold',
      label: 'Cold',
      group: 'colorize',
      apply: image => image.photoFilter(PhotoFilterPreset.Cooling, 0.35),
    },
    {
      kind: 'action',
      key: 'action-colorize-sunny',
      label: 'Sunny',
      group: 'colorize',
      apply: image => {
        image.photoFilter(PhotoFilterPreset.Warming, 0.2);
        image.vibrance(20);
        image.applyEffect(EffectSpec.Brightness.new({ amount: 8 }));
      },
    },
    {
      kind: 'action',
      key: 'action-colorize-cloudy',
      label: 'Cloudy',
      group: 'colorize',
      apply: image => {
        image.photoFilter(PhotoFilterPreset.Cooling, 0.2);
        image.applyEffect(EffectSpec.Saturation.new({ amount: -20 }));
        image.applyEffect(EffectSpec.Brightness.new({ amount: -6 }));
      },
    },
    {
      kind: 'action',
      key: 'action-colorize-sepia',
      label: 'Sepia',
      group: 'colorize',
      apply: image => image.photoFilter(PhotoFilterPreset.Sepia, 0.35),
    },
    {
      kind: 'action',
      key: 'action-colorize-vibrant',
      label: 'Vibrant',
      group: 'colorize',
      apply: image => {
        image.applyEffect(EffectSpec.Saturation.new({ amount: 30 }));
        image.vibrance(30);
      },
    },
    {
      kind: 'action',
      key: 'action-colorize-fade',
      label: 'Fade',
      group: 'colorize',
      apply: image => {
        image.applyEffect(EffectSpec.Contrast.new({ amount: -20 }));
        image.applyEffect(EffectSpec.Saturation.new({ amount: -15 }));
      },
    },
    {
      kind: 'action',
      key: 'action-colorize-moody',
      label: 'Moody',
      group: 'colorize',
      apply: image => {
        image.photoFilter(PhotoFilterPreset.Cooling, 0.25);
        image.applyEffect(EffectSpec.Contrast.new({ amount: 15 }));
        image.applyEffect(EffectSpec.Brightness.new({ amount: -10 }));
      },
    },
    {
      kind: 'action',
      key: 'action-colorize-golden-hour',
      label: 'Golden Hour',
      group: 'colorize',
      apply: image => {
        image.photoFilter(PhotoFilterPreset.Warming, 0.45);
        image.applyEffect(EffectSpec.Brightness.new({ amount: 12 }));
      },
    },
    {
      kind: 'action',
      key: 'action-colorize-dreamy',
      label: 'Dreamy',
      group: 'colorize',
      apply: image => {
        image.photoFilter(PhotoFilterPreset.Warming, 0.15);
        image.applyEffect(EffectSpec.Contrast.new({ amount: -15 }));
        image.applyEffect(EffectSpec.Saturation.new({ amount: -10 }));
      },
    },
    {
      kind: 'action',
      key: 'action-colorize-noir',
      label: 'Noir',
      group: 'colorize',
      apply: image => {
        // image.grayscale();
        image.applyEffect(EffectSpec.Saturation.new({ amount: -100 }));
        image.applyEffect(EffectSpec.Contrast.new({ amount: 20 }));
      },
    },
    {
      kind: 'action',
      key: 'action-colorize-vintage',
      label: 'Vintage',
      group: 'colorize',
      apply: image => {
        image.photoFilter(PhotoFilterPreset.Sepia, 0.2);
        image.applyEffect(EffectSpec.Contrast.new({ amount: -10 }));
        image.applyEffect(EffectSpec.Saturation.new({ amount: -20 }));
      },
    },
    {
      kind: 'action',
      key: 'action-colorize-ocean',
      label: 'Ocean',
      group: 'colorize',
      apply: image => image.photoFilter(PhotoFilterPreset.Underwater, 0.35),
    },
    {
      kind: 'action',
      key: 'action-colorize-forest',
      label: 'Forest',
      group: 'colorize',
      apply: image => image.photoFilter(PhotoFilterPreset.DeepEmerald, 0.3),
    },
    {
      kind: 'action',
      key: 'action-colorize-frost',
      label: 'Frost',
      group: 'colorize',
      apply: image => image.photoFilter(PhotoFilterPreset.CoolingDark, 0.3),
    },
    {
      kind: 'action',
      key: 'action-colorize-neon',
      label: 'Neon',
      group: 'colorize',
      apply: image => image.photoFilter(PhotoFilterPreset.Magenta, 0.25),
    },
  ],
}