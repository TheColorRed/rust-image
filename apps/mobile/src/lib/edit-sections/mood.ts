import { EffectSpec, FilterType } from "@alakazam/mobile";
import { EditSection } from "../edit-sections";

// Ordered by how often people tend to reach for each mood, most popular first. Each mood is its `live` effects, which
// run in order for the instant preview and are what the replay applies.
export const colorization: EditSection = {
  key: 'section-mood',
  label: 'Mood',
  previewThumbnails: true,
  controls: [
    {
      kind: 'action',
      key: 'action-mood-vibrant',
      label: 'Vibrant',
      group: 'mood',
      live: () => [
        EffectSpec.Saturation.new({ amount: 30 }),
        EffectSpec.Vibrance.new({ vibrance: 30, saturation: 0 }),
      ],
    },
    {
      kind: 'action',
      key: 'action-mood-warm',
      label: 'Warm',
      group: 'mood',
      live: () => EffectSpec.PhotoFilter.new({ filter: FilterType.WarmingLight, density: 0.35, preserveLuminosity: false }),
    },
    {
      kind: 'action',
      key: 'action-mood-golden-hour',
      label: 'Golden Hour',
      group: 'mood',
      live: () => [
        EffectSpec.PhotoFilter.new({ filter: FilterType.WarmingLight, density: 0.45, preserveLuminosity: false }),
        EffectSpec.Brightness.new({ amount: 12 }),
      ],
    },
    {
      kind: 'action',
      key: 'action-mood-vintage',
      label: 'Vintage',
      group: 'mood',
      live: () => [
        EffectSpec.PhotoFilter.new({ filter: FilterType.Sepia, density: 0.2, preserveLuminosity: false }),
        EffectSpec.Contrast.new({ amount: -10 }),
        EffectSpec.Saturation.new({ amount: -20 }),
      ],
    },
    {
      kind: 'action',
      key: 'action-mood-noir',
      label: 'Noir',
      group: 'mood',
      live: () => [
        EffectSpec.Saturation.new({ amount: -100 }),
        EffectSpec.Contrast.new({ amount: 20 }),
      ],
    },
    {
      kind: 'action',
      key: 'action-mood-moody',
      label: 'Moody',
      group: 'mood',
      live: () => [
        EffectSpec.PhotoFilter.new({ filter: FilterType.CoolingLight, density: 0.25, preserveLuminosity: false }),
        EffectSpec.Contrast.new({ amount: 15 }),
        EffectSpec.Brightness.new({ amount: -10 }),
      ],
    },
    {
      kind: 'action',
      key: 'action-mood-sunny',
      label: 'Sunny',
      group: 'mood',
      live: () => [
        EffectSpec.PhotoFilter.new({ filter: FilterType.WarmingLight, density: 0.2, preserveLuminosity: false }),
        EffectSpec.Vibrance.new({ vibrance: 20, saturation: 0 }),
        EffectSpec.Brightness.new({ amount: 8 }),
      ],
    },
    {
      kind: 'action',
      key: 'action-mood-cold',
      label: 'Cold',
      group: 'mood',
      live: () => EffectSpec.PhotoFilter.new({ filter: FilterType.CoolingLight, density: 0.35, preserveLuminosity: false }),
    },
    {
      kind: 'action',
      key: 'action-mood-fade',
      label: 'Fade',
      group: 'mood',
      live: () => [
        EffectSpec.Contrast.new({ amount: -20 }),
        EffectSpec.Saturation.new({ amount: -15 }),
      ],
    },
    {
      kind: 'action',
      key: 'action-mood-sepia',
      label: 'Sepia',
      group: 'mood',
      live: () => EffectSpec.PhotoFilter.new({ filter: FilterType.Sepia, density: 0.35, preserveLuminosity: false }),
    },
    {
      kind: 'action',
      key: 'action-mood-dreamy',
      label: 'Dreamy',
      group: 'mood',
      live: () => [
        EffectSpec.PhotoFilter.new({ filter: FilterType.WarmingLight, density: 0.15, preserveLuminosity: false }),
        EffectSpec.Contrast.new({ amount: -15 }),
        EffectSpec.Saturation.new({ amount: -10 }),
      ],
    },
    {
      kind: 'action',
      key: 'action-mood-cloudy',
      label: 'Cloudy',
      group: 'mood',
      live: () => [
        EffectSpec.PhotoFilter.new({ filter: FilterType.CoolingLight, density: 0.2, preserveLuminosity: false }),
        EffectSpec.Saturation.new({ amount: -20 }),
        EffectSpec.Brightness.new({ amount: -6 }),
      ],
    },
    {
      kind: 'action',
      key: 'action-mood-midnight',
      label: 'Midnight',
      group: 'mood',
      live: () => [
        EffectSpec.PhotoFilter.new({ filter: FilterType.CoolingDark, density: 0.4, preserveLuminosity: false }),
        EffectSpec.Brightness.new({ amount: -20 }),
        EffectSpec.Contrast.new({ amount: 15 }),
        EffectSpec.Saturation.new({ amount: -10 }),
      ],
    },
    {
      kind: 'action',
      key: 'action-mood-ocean',
      label: 'Ocean',
      group: 'mood',
      live: () => EffectSpec.PhotoFilter.new({ filter: FilterType.Underwater, density: 0.35, preserveLuminosity: false }),
    },
    {
      kind: 'action',
      key: 'action-mood-forest',
      label: 'Forest',
      group: 'mood',
      live: () => EffectSpec.PhotoFilter.new({ filter: FilterType.DeepEmerald, density: 0.3, preserveLuminosity: false }),
    },
    {
      kind: 'action',
      key: 'action-mood-frost',
      label: 'Frost',
      group: 'mood',
      live: () => EffectSpec.PhotoFilter.new({ filter: FilterType.CoolingDark, density: 0.3, preserveLuminosity: false }),
    },
    {
      kind: 'action',
      key: 'action-mood-neon',
      label: 'Neon',
      group: 'mood',
      live: () => EffectSpec.PhotoFilter.new({ filter: FilterType.Magenta, density: 0.25, preserveLuminosity: false }),
    },
  ],
}
