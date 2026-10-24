import { glitchArt } from '@/src/effects/glitch-art';
import { SliderControl } from '@/src/lib/project/edit-sections';

export const glitchArtControl = {
  kind: 'slider',
  key: 'action-glitch-art',
  label: 'Glitch Art',
  sliderLabel: ['Strength', 'Horizontal', 'Vertical'],
  display: 'tabulated',
  min: [0, -20, -20],
  max: [1, 20, 20],
  step: [0.01, 1, 1],
  defaultValue: [1, -10, 0],
  applyOnSelect: true,
  tool: (photo, [strength, offsetX, offsetY]) => glitchArt(photo, strength, offsetX, offsetY),
  thumb: photo => glitchArt(photo, 1, -5, 0),
} satisfies SliderControl<number>;
