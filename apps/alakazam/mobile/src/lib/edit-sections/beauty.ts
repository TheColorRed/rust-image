import { EffectSpec } from '@alakazam/mobile';
import { type EditSection } from '../edit-sections';
import { applyEyeMakeup, makeupMasks } from '../eye-makeup';
import { applyLipstick } from '../lipstick';
import {
  BLEMISH_TOOL_KEY,
  BROW_BONE_KEY,
  EYELINER_KEY,
  EYESHADOW_KEY,
  LIPSTICK_KEY,
  MAKEUP_GROUP_KEY,
  RED_EYE_TOOL_KEY,
  SKIN_SMOOTH_KEY,
  SKIN_TAN_KEY,
  SKIN_TONE_KEY,
  UNDER_EYE_KEY,
} from './keys';

export const beauty: EditSection = {
  key: 'section-beauty',
  label: 'Beauty',
  controls: [
    {
      kind: 'slider',
      key: SKIN_SMOOTH_KEY,
      label: 'Skin Smooth',
      min: 0,
      // Up to 1 is how much of the smoothing shows; past 1 the smoothing itself gets stronger, up to 3.
      max: 3,
      step: 0.1,
      defaultValue: 0,
      applyOnSelect: false,
      live: value => EffectSpec.SkinSmooth.new({ amount: value }),
    },
    {
      kind: 'slider',
      key: SKIN_TAN_KEY,
      label: 'Skin Tan',
      // Where on the tan scale: none at 0, the darkest at 1.
      min: 0,
      max: 1,
      step: 0.01,
      defaultValue: 0,
      applyOnSelect: false,
      picker: 'skin-tan',
      live: value => EffectSpec.SkinTan.new({ offset: value }),
    },
    {
      kind: 'slider',
      key: SKIN_TONE_KEY,
      label: 'Skin Tone',
      // Negative lightens the original skin; positive darkens it.
      min: -400,
      max: 400,
      step: 1,
      defaultValue: 0,
      applyOnSelect: false,
      live: value => EffectSpec.SkinTone.new({ amount: value }),
    },
    { kind: 'tool', key: RED_EYE_TOOL_KEY, label: 'Red Eye' },
    { kind: 'tool', key: BLEMISH_TOOL_KEY, label: 'Blemish' },
    {
      kind: 'group',
      key: MAKEUP_GROUP_KEY,
      label: 'Makeup',
      controls: [
        {
          kind: 'slider',
          key: EYELINER_KEY,
          label: 'Eyeliner',
          display: 'tabulated',
          sliderLabel: ['Color', 'Strength'],
          min: [0, 0],
          max: [1, 1],
          step: [0.01, 0.01],
          defaultValue: [0, 0.5],
          picker: ['hue', undefined],
          swatches: [
            { label: 'Black', color: '#111111', values: [0, 0.65] },
            { label: 'Brown', color: '#553126', values: [1 - 14 / 360, 0.55] },
            { label: 'Navy', color: '#202C50', values: [1 - 227 / 360, 0.55] },
          ],
          preload: makeupMasks,
          tool: (photo, [hue, strength]: number[], context) =>
            applyEyeMakeup('eyeliner', photo, hue, strength, 0, 0.07, 1, context),
        },
        {
          kind: 'slider',
          key: EYESHADOW_KEY,
          label: 'Eyeshadow',
          display: 'tabulated',
          sliderLabel: ['Color', 'Strength', 'Vividness', 'Brightness', 'Fullness'],
          min: [0, 0, 0, 0, 0.3],
          max: [1, 1, 1, 1, 3],
          step: [0.01, 0.01, 0.01, 0.01, 0.01],
          defaultValue: [1 - 24 / 360, 0.45, 0.45, 0.55, 1],
          picker: ['hue', undefined, undefined, undefined, undefined],
          swatches: [
            { label: 'Champagne', color: '#D8B68A', values: [1 - 34 / 360, undefined, 0.36, 0.85] },
            { label: 'Bronze', color: '#A66B3F', values: [1 - 26 / 360, undefined, 0.62, 0.65] },
            { label: 'Rose', color: '#B97882', values: [1 - 351 / 360, undefined, 0.35, 0.73] },
            { label: 'Plum', color: '#70445F', values: [1 - 324 / 360, undefined, 0.39, 0.44] },
            { label: 'Smoky', color: '#55515A', values: [1 - 260 / 360, undefined, 0.1, 0.35] },
          ],
          preload: makeupMasks,
          tool: (photo, [hue, strength, saturation, brightness, fullness]: number[], context) =>
            applyEyeMakeup('eyeshadow', photo, hue, strength, saturation, brightness, fullness, context),
        },
        {
          kind: 'slider',
          key: BROW_BONE_KEY,
          label: 'Brow Bone',
          display: 'tabulated',
          sliderLabel: ['Color', 'Strength', 'Vividness', 'Brightness', 'Fullness'],
          min: [0, 0, 0, 0, 0.3],
          max: [1, 1, 1, 1, 3],
          step: [0.01, 0.01, 0.01, 0.01, 0.01],
          defaultValue: [1 - 38 / 360, 0.8, 0.4, 0.9, 1],
          picker: ['hue', undefined, undefined, undefined, undefined],
          swatches: [
            { label: 'Pearl', color: '#F2E8DC', values: [1 - 32 / 360, undefined, 0.09, 0.95] },
            { label: 'Champagne', color: '#DFC49A', values: [1 - 38 / 360, undefined, 0.31, 0.87] },
            { label: 'Soft gold', color: '#D3AD68', values: [1 - 39 / 360, undefined, 0.5, 0.83] },
            { label: 'Rose glow', color: '#D6A5A3', values: [1 - 1 / 360, undefined, 0.23, 0.84] },
            { label: 'Bronze glow', color: '#AA754D', values: [1 - 26 / 360, undefined, 0.55, 0.67] },
          ],
          preload: makeupMasks,
          tool: (photo, [hue, strength, saturation, brightness, fullness]: number[], context) =>
            applyEyeMakeup('brow-bone', photo, hue, strength, saturation, brightness, fullness, context),
        },
        {
          kind: 'slider',
          key: UNDER_EYE_KEY,
          label: 'Under Eye',
          display: 'tabulated',
          sliderLabel: ['Color', 'Strength', 'Vividness', 'Brightness', 'Fullness'],
          min: [0, 0, 0, 0, 0.3],
          max: [1, 1, 1, 1, 3],
          step: [0.01, 0.01, 0.01, 0.01, 0.01],
          defaultValue: [1 - 24 / 360, 0.28, 0.22, 0.88, 1],
          picker: ['hue', undefined, undefined, undefined, undefined],
          swatches: [
            { label: 'Brighten', color: '#EFD9CA', values: [1 - 24 / 360, undefined, 0.15, 0.94] },
            { label: 'Light peach', color: '#EDB79F', values: [1 - 16 / 360, undefined, 0.33, 0.93] },
            { label: 'Peach', color: '#E89A78', values: [1 - 16 / 360, undefined, 0.48, 0.91] },
            { label: 'Pink', color: '#DEA1AB', values: [1 - 350 / 360, undefined, 0.28, 0.87] },
            { label: 'Orange', color: '#D9784D', values: [1 - 18 / 360, undefined, 0.64, 0.85] },
          ],
          preload: makeupMasks,
          tool: (photo, [hue, strength, saturation, brightness, fullness]: number[], context) =>
            applyEyeMakeup('under-eye', photo, hue, strength, saturation, brightness, fullness, context),
        },
        {
          kind: 'slider',
          key: LIPSTICK_KEY,
          label: 'Lipstick',
          // First the color (a swatch or the bar), then how much of it, how soft the edge is, and for a custom color how vivid and bright.
          display: 'tabulated',
          sliderLabel: ['Color', 'Strength', 'Softness', 'Vividness', 'Brightness'],
          min: [0, 0, 0, 0, 0],
          max: [1, 1, 1, 1, 1],
          step: [0.01, 0.01, 0.01, 0.01, 0.01],
          // Opening the control puts this lipstick on at once (red, lightly), so there is something to change.
          defaultValue: [0, 0.2, 1, 1, 1],
          picker: ['hue', undefined, undefined, undefined, undefined],
          // Colors people ask for by name, from a shade chart of real lipsticks (each color is read from the chart; its name is the
          // plain description of it). Shades that were nearly the same color (a color distance under about 8) are left out, so every
          // swatch is a choice a person can see. A swatch is a hue on the bar (1 minus its degrees around the wheel over 360, since the bar runs
          // backwards), how vivid it is and how bright. A "sheer" shade also sets a light strength. Black and white come first.
          swatches: [
            { label: 'Black', color: '#000000', values: [0, undefined, undefined, 0, 0] },
            { label: 'White', color: '#FFFFFF', values: [0, undefined, undefined, 0, 1] },
            // Nudes.
            { label: 'Creamy nude', color: '#EBAD81', values: [1 - 24.9 / 360, undefined, undefined, 0.45, 0.92] },
            { label: 'Nude peach', color: '#F18E6A', values: [1 - 16.0 / 360, undefined, undefined, 0.56, 0.95] },
            { label: 'Nude', color: '#DD7C3D', values: [1 - 23.6 / 360, undefined, undefined, 0.72, 0.87] },
            { label: 'Caramel nude', color: '#B87A74', values: [1 - 5.3 / 360, undefined, undefined, 0.37, 0.72] },
            { label: 'Dark nude', color: '#D27754', values: [1 - 16.8 / 360, undefined, undefined, 0.6, 0.82] },
            { label: 'Coffee nude', color: '#B46F4B', values: [1 - 20.6 / 360, undefined, undefined, 0.58, 0.71] },
            // Pinks.
            { label: 'Pale pink', color: '#F096BD', values: [1 - 334.0 / 360, undefined, undefined, 0.37, 0.94] },
            { label: 'Nude pink', color: '#E9A8C6', values: [1 - 332.3 / 360, undefined, undefined, 0.28, 0.91] },
            { label: 'Pale rose pink', color: '#EC7EAA', values: [1 - 336.0 / 360, undefined, undefined, 0.47, 0.93] },
            { label: 'Rose pink', color: '#DF4A86', values: [1 - 335.7 / 360, undefined, undefined, 0.67, 0.87] },
            { label: 'Browny pink', color: '#C06D7E', values: [1 - 347.7 / 360, undefined, undefined, 0.43, 0.75] },
            { label: 'Dark dusky pink', color: '#AE5373', values: [1 - 338.8 / 360, undefined, undefined, 0.52, 0.68] },
            { label: 'Dark pink', color: '#BC316A', values: [1 - 335.4 / 360, undefined, undefined, 0.74, 0.74] },
            { label: 'Deep rose pink', color: '#E32957', values: [1 - 345.2 / 360, undefined, undefined, 0.82, 0.89] },
            { label: 'Bright fuchsia', color: '#E30274', values: [1 - 329.5 / 360, undefined, undefined, 0.99, 0.89] },
            // Corals and oranges.
            { label: 'Sheer coral', color: '#EE8372', values: [1 - 8.2 / 360, 0.45, undefined, 0.52, 0.93] },
            {
              label: 'Sheer strawberry pink',
              color: '#E74957',
              values: [1 - 354.7 / 360, 0.45, undefined, 0.68, 0.91],
            },
            { label: 'Coral red', color: '#E73C42', values: [1 - 357.9 / 360, undefined, undefined, 0.74, 0.91] },
            { label: 'Deep mandarin', color: '#E6421A', values: [1 - 11.8 / 360, undefined, undefined, 0.89, 0.9] },
            // Reds.
            { label: 'Bright red', color: '#E0011B', values: [1 - 353.0 / 360, undefined, undefined, 1.0, 0.88] },
            { label: 'Classic red', color: '#CD0215', values: [1 - 354.4 / 360, undefined, undefined, 0.99, 0.8] },
            { label: 'Deep red', color: '#BF012F', values: [1 - 345.5 / 360, undefined, undefined, 0.99, 0.75] },
            // Plums and berries.
            { label: 'Sheer plum', color: '#C58396', values: [1 - 342.7 / 360, 0.45, undefined, 0.34, 0.77] },
            { label: 'Golden plum', color: '#9E4776', values: [1 - 327.6 / 360, undefined, undefined, 0.55, 0.62] },
            { label: 'Plum', color: '#A6315B', values: [1 - 338.5 / 360, undefined, undefined, 0.7, 0.65] },
            { label: 'Raspberry pink', color: '#B00048', values: [1 - 335.5 / 360, undefined, undefined, 1.0, 0.69] },
            { label: 'Burgundy', color: '#7F023E', values: [1 - 331.2 / 360, undefined, undefined, 0.98, 0.5] },
            {
              label: 'Deep purple plum',
              color: '#8C0364',
              values: [1 - 317.5 / 360, undefined, undefined, 0.98, 0.55],
            },
          ],
          // The AI finds the lips once, when the control opens; dragging only changes the color, the strength or the edge.
          preload: makeupMasks,
          tool: (photo, [hue, amount, softness, saturation, brightness]: number[], context) =>
            applyLipstick(photo, hue, amount, softness, saturation, brightness, context),
        },
      ],
    },
  ],
};
