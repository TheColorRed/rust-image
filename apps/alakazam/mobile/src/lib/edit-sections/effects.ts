import { vignette } from '@/src/effects/vignette';
import { autoColor, autoTone, grayscale, invert, posterize, threshold } from '@abra/core/adjustments';
import { EditSection, SliderControl } from '../edit-sections';

export const effects: EditSection = {
  key: 'section-effects',
  label: 'Effects',
  previewThumbnails: true,
  controls: [
    {
      kind: 'action',
      key: 'action-grayscale',
      label: 'Grayscale',
      thumb: photo => grayscale().apply(photo),
      tool: photo => grayscale().apply(photo),
    },
    {
      kind: 'action',
      key: 'action-invert',
      label: 'Invert',
      thumb: photo => invert().apply(photo),
      tool: photo => invert().apply(photo),
    },
    {
      kind: 'action',
      key: 'action-auto-tone',
      label: 'Auto Tone',
      tool: photo => autoTone().apply(photo),
      thumb: photo => autoTone().apply(photo),
    },
    {
      kind: 'action',
      key: 'action-auto-color',
      label: 'Auto Color',
      tool: photo => autoColor().apply(photo),
      thumb: photo => autoColor().apply(photo),
    },
    {
      kind: 'slider',
      key: 'action-threshold',
      label: 'Threshold',
      max: 255,
      defaultValue: 128,
      tool: (photo, [value]) => threshold(value).apply(photo),
      thumb: photo => threshold(128).apply(photo),
    } satisfies SliderControl<number>,
    {
      kind: 'slider',
      key: 'action-vignette',
      label: 'Vignette',
      applyOnSelect: false,
      tool: (photo, [value]) => vignette(photo, value / 100),
      thumb: photo => vignette(photo, 1),
    } satisfies SliderControl<number>,
    {
      kind: 'slider',
      key: 'action-posterize',
      label: 'Posterize',
      max: 10,
      defaultValue: 5,
      tool: (photo, [value]) => posterize(value).apply(photo),
      thumb: photo => posterize(5).apply(photo),
    } satisfies SliderControl<number>,
  ],
};
