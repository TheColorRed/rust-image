import { brightness, contrast, FilterType, PhotoFilter, photoFilter, saturation, vibrance } from '@abra/core/adjustments';
import { EditSection } from '@/src/lib/project/edit-sections';

// Ordered by how often people tend to reach for each mood, most popular first. Each mood is its `tool`, which runs its
// adjustments in order for the instant preview and is what the replay applies.
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
      deferred: true,
      tool: photo => {
        saturation(30).applyDeferred(photo);
        vibrance(30).applyDeferred(photo);
      },
    },
    {
      kind: 'action',
      key: 'action-mood-warm',
      label: 'Warm',
      group: 'mood',
      deferred: true,
      tool: photo => {
        photoFilter(PhotoFilter.Preset.new(FilterType.WarmingLight)).withDensity(0.35).withPreserveLuminosity(false).applyDeferred(photo);
      },
    },
    {
      kind: 'action',
      key: 'action-mood-golden-hour',
      label: 'Golden Hour',
      group: 'mood',
      deferred: true,
      tool: photo => {
        photoFilter(PhotoFilter.Preset.new(FilterType.WarmingLight)).withDensity(0.45).withPreserveLuminosity(false).applyDeferred(photo);
        brightness(12).applyDeferred(photo);
      },
    },
    {
      kind: 'action',
      key: 'action-mood-vintage',
      label: 'Vintage',
      group: 'mood',
      deferred: true,
      tool: photo => {
        photoFilter(PhotoFilter.Preset.new(FilterType.Sepia)).withDensity(0.2).withPreserveLuminosity(false).applyDeferred(photo);
        contrast(-10).applyDeferred(photo);
        saturation(-20).applyDeferred(photo);
      },
    },
    {
      kind: 'action',
      key: 'action-mood-noir',
      label: 'Noir',
      group: 'mood',
      deferred: true,
      tool: photo => {
        saturation(-100).applyDeferred(photo);
        contrast(20).applyDeferred(photo);
      },
    },
    {
      kind: 'action',
      key: 'action-mood-moody',
      label: 'Moody',
      group: 'mood',
      deferred: true,
      tool: photo => {
        photoFilter(PhotoFilter.Preset.new(FilterType.CoolingLight)).withDensity(0.25).withPreserveLuminosity(false).applyDeferred(photo);
        contrast(15).applyDeferred(photo);
        brightness(-10).applyDeferred(photo);
      },
    },
    {
      kind: 'action',
      key: 'action-mood-sunny',
      label: 'Sunny',
      group: 'mood',
      deferred: true,
      tool: photo => {
        photoFilter(PhotoFilter.Preset.new(FilterType.WarmingLight)).withDensity(0.2).withPreserveLuminosity(false).applyDeferred(photo);
        vibrance(20).applyDeferred(photo);
        brightness(8).applyDeferred(photo);
      },
    },
    {
      kind: 'action',
      key: 'action-mood-cold',
      label: 'Cold',
      group: 'mood',
      deferred: true,
      tool: photo => {
        photoFilter(PhotoFilter.Preset.new(FilterType.CoolingLight)).withDensity(0.35).withPreserveLuminosity(false).applyDeferred(photo);
      },
    },
    {
      kind: 'action',
      key: 'action-mood-fade',
      label: 'Fade',
      group: 'mood',
      deferred: true,
      tool: photo => {
        contrast(-20).applyDeferred(photo);
        saturation(-15).applyDeferred(photo);
      },
    },
    {
      kind: 'action',
      key: 'action-mood-sepia',
      label: 'Sepia',
      group: 'mood',
      deferred: true,
      tool: photo => {
        photoFilter(PhotoFilter.Preset.new(FilterType.Sepia)).withDensity(0.35).withPreserveLuminosity(false).applyDeferred(photo);
      },
    },
    {
      kind: 'action',
      key: 'action-mood-dreamy',
      label: 'Dreamy',
      group: 'mood',
      deferred: true,
      tool: photo => {
        photoFilter(PhotoFilter.Preset.new(FilterType.WarmingLight)).withDensity(0.15).withPreserveLuminosity(false).applyDeferred(photo);
        contrast(-15).applyDeferred(photo);
        saturation(-10).applyDeferred(photo);
      },
    },
    {
      kind: 'action',
      key: 'action-mood-cloudy',
      label: 'Cloudy',
      group: 'mood',
      deferred: true,
      tool: photo => {
        photoFilter(PhotoFilter.Preset.new(FilterType.CoolingLight)).withDensity(0.2).withPreserveLuminosity(false).applyDeferred(photo);
        saturation(-20).applyDeferred(photo);
        brightness(-6).applyDeferred(photo);
      },
    },
    {
      kind: 'action',
      key: 'action-mood-midnight',
      label: 'Midnight',
      group: 'mood',
      deferred: true,
      tool: photo => {
        photoFilter(PhotoFilter.Preset.new(FilterType.CoolingDark)).withDensity(0.4).withPreserveLuminosity(false).applyDeferred(photo);
        brightness(-20).applyDeferred(photo);
        contrast(15).applyDeferred(photo);
        saturation(-10).applyDeferred(photo);
      },
    },
    {
      kind: 'action',
      key: 'action-mood-ocean',
      label: 'Ocean',
      group: 'mood',
      deferred: true,
      tool: photo => {
        photoFilter(PhotoFilter.Preset.new(FilterType.Underwater)).withDensity(0.35).withPreserveLuminosity(false).applyDeferred(photo);
      },
    },
    {
      kind: 'action',
      key: 'action-mood-forest',
      label: 'Forest',
      group: 'mood',
      deferred: true,
      tool: photo => {
        photoFilter(PhotoFilter.Preset.new(FilterType.DeepEmerald)).withDensity(0.3).withPreserveLuminosity(false).applyDeferred(photo);
      },
    },
    {
      kind: 'action',
      key: 'action-mood-frost',
      label: 'Frost',
      group: 'mood',
      deferred: true,
      tool: photo => {
        photoFilter(PhotoFilter.Preset.new(FilterType.CoolingDark)).withDensity(0.3).withPreserveLuminosity(false).applyDeferred(photo);
      },
    },
    {
      kind: 'action',
      key: 'action-mood-neon',
      label: 'Neon',
      group: 'mood',
      deferred: true,
      tool: photo => {
        photoFilter(PhotoFilter.Preset.new(FilterType.Magenta)).withDensity(0.25).withPreserveLuminosity(false).applyDeferred(photo);
      },
    },
  ],
};
