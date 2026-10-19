import { bleachBypass } from '@/src/effects/bleach-bypass';
import { chiaroscuro } from '@/src/effects/chiaroscuro';
import { colorSplash } from '@/src/effects/color-splash';
import { cyanotype } from '@/src/effects/cyanotype';
import { fadedFilm } from '@/src/effects/faded-film';
import { infrared } from '@/src/effects/infrared';
import { splitTone } from '@/src/effects/split-tone';
import { EditSection, SliderControl } from '@/src/lib/project/edit-sections';
import { colorAreaKey } from '@/src/lib/project/edit-sections/keys';

export const specialEffects: EditSection = {
  key: 'section-special-effects',
  label: 'Special Effects',
  previewThumbnails: true,
  controls: [
    {
      kind: 'slider',
      key: 'action-color-splash',
      label: 'Color Splash',
      display: 'tabulated',
      sliderLabel: ['Color 1', 'Color 2', 'Color 3', 'Range'],
      pillFormat: [undefined, undefined, undefined, 'plain'],
      min: [0, 0, 0, 5],
      max: [1, 1, 1, 90],
      step: [0.01, 0.01, 0.01, 1],
      defaultValue: [1, 0, 0, 40],
      picker: ['hue', 'hue', 'hue', undefined],
      applyOnSelect: false,
      areaKeys: [colorAreaKey(0), colorAreaKey(1), colorAreaKey(2)],
      tool: (photo, [first, second, third, range]: number[], context) => {
        colorSplash(
          photo,
          // A color that is off is left out, but keeps its place so its drawn area still matches it.
          [first, second, third].flatMap((v, index) =>
            v > 0 ? [{ hue: 360 * (1 - v), area: context.areas?.[index] }] : [],
          ),
          range,
        );
      },
      thumb: photo => colorSplash(photo, [{ hue: 0 }], 40),
    },
    {
      kind: 'slider',
      key: 'action-infrared',
      label: 'Infrared',
      min: 0,
      max: 1,
      step: 0.01,
      defaultValue: 0,
      picker: 'infrared',
      tool: (photo, [value]) => infrared(photo, value),
      thumb: photo => infrared(photo, 0),
    } satisfies SliderControl<number>,
    {
      kind: 'slider',
      key: 'action-chiaroscuro',
      label: 'Chiaroscuro',
      sliderLabel: ['Strength', 'Color'],
      min: [0, 0],
      max: [1, 1],
      step: [0.001, 0.001],
      defaultValue: [0.5, 1],
      swatches: [
        { label: 'Color', color: '#d9a88a', values: [undefined, 1] },
        { label: 'Black and white', color: '#7a7a7a', values: [undefined, 0] },
      ],
      applyOnSelect: true,
      tool: (photo, [amount, color]) => chiaroscuro(photo, amount, color),
      thumb: photo => chiaroscuro(photo, 1),
    } satisfies SliderControl<number>,
    {
      kind: 'slider',
      key: 'action-cyanotype',
      label: 'Cyanotype',
      min: 0,
      max: 1,
      step: 0.01,
      defaultValue: 1,
      applyOnSelect: true,
      tool: (photo, [amount]: number[]) => cyanotype(photo, amount),
      thumb: photo => cyanotype(photo, 1),
    } satisfies SliderControl<number>,
    {
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
    } satisfies SliderControl<number>,
    {
      kind: 'slider',
      key: 'action-bleach-bypass',
      label: 'Bleach Bypass',
      min: 0,
      max: 1,
      step: 0.01,
      defaultValue: 0.6,
      applyOnSelect: true,
      tool: (photo, [amount]: number[]) => bleachBypass(photo, amount),
      thumb: photo => bleachBypass(photo, 1),
    } satisfies SliderControl<number>,
    {
      kind: 'slider',
      key: 'action-split-tone',
      label: 'Split Tone',
      min: 0,
      max: 1,
      step: 0.01,
      defaultValue: 0.6,
      applyOnSelect: true,
      tool: (photo, [amount]: number[]) => splitTone(photo, amount),
      thumb: photo => splitTone(photo, 1),
    } satisfies SliderControl<number>,
  ],
};
