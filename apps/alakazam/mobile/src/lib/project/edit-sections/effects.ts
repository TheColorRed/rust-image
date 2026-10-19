import { vignette } from '@/src/effects/vignette';
import { autoColor, autoTone, grayscale, invert, posterize, threshold } from '@abra/core/adjustments';
import { EditSection, SliderControl } from '@/src/lib/project/edit-sections';

export const effects: EditSection = {
  key: 'section-effects',
  label: 'Effects',
  previewThumbnails: true,
  controls: [
    {
      kind: 'action',
      key: 'action-grayscale',
      label: 'Black & White',
      deferred: true,
      thumb: photo => grayscale().apply(photo),
      tool: photo => grayscale().applyDeferred(photo),
    },
    {
      kind: 'action',
      key: 'action-invert',
      label: 'Flip Colors',
      deferred: true,
      thumb: photo => invert().apply(photo),
      tool: photo => invert().applyDeferred(photo),
    },
    {
      kind: 'action',
      key: 'action-auto-tone',
      label: 'Auto Fix',
      tool: photo => autoTone().apply(photo),
      thumb: photo => autoTone().apply(photo),
    },
    {
      kind: 'action',
      key: 'action-auto-color',
      label: 'Auto Colors',
      tool: photo => autoColor().apply(photo),
      thumb: photo => autoColor().apply(photo),
    },
    {
      kind: 'slider',
      key: 'action-threshold',
      label: 'Black & White Only',
      max: 255,
      defaultValue: 128,
      deferred: true,
      tool: (photo, [value]) => threshold(value).applyDeferred(photo),
      thumb: photo => threshold(128).apply(photo),
    } satisfies SliderControl<number>,
    {
      kind: 'slider',
      key: 'action-vignette',
      label: 'Dark Corners',
      applyOnSelect: false,
      tool: (photo, [value]) => vignette(photo, value / 100),
      thumb: photo => vignette(photo, 1),
    } satisfies SliderControl<number>,
    {
      kind: 'slider',
      key: 'action-posterize',
      label: 'Flat Colors',
      max: 10,
      defaultValue: 5,
      deferred: true,
      tool: (photo, [value]) => posterize(value).applyDeferred(photo),
      thumb: photo => posterize(5).apply(photo),
    } satisfies SliderControl<number>,
  ],
};
