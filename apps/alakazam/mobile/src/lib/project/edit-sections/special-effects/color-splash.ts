import { colorSplash } from '@/src/effects/color-splash';
import { SliderControl } from '@/src/lib/project/edit-sections';
import { colorAreaKey } from '@/src/lib/project/edit-sections/keys';

export const colorSplashControl = {
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
  applyOnSelect: true,
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
} satisfies SliderControl<number>;
