import { fadedFilm } from '@/src/effects/faded-film';
import { SliderControl } from '@/src/lib/project/edit-sections';

export const fadedFilmControl = {
  kind: 'slider',
  key: 'action-faded-film',
  label: 'Faded Film',
  display: 'tabulated',
  sliderLabel: ['Strength', 'Warmth', 'Fade'],
  pillFormat: ['percent', 'signed', 'percent'],
  min: [0, 0, 0],
  max: [1, 1, 1],
  step: [0.01, 0.01, 0.01],
  defaultValue: [1, 0.5, 0.5],
  applyOnSelect: true,
  tool: (photo, [strength, warmth, fade]: number[]) => fadedFilm(photo, strength, warmth, fade),
  thumb: photo => fadedFilm(photo, 1, 0.5, 0.5),
} satisfies SliderControl<number>;
