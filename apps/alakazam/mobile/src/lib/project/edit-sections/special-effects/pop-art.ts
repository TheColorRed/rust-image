import { subjectMask } from '@/src/lib/ai/subject';
import { POP_ART_COLORS, POP_ART_DEFAULTS, popArt, popArtSwatches, type PopArtOptions } from '@/src/effects/pop-art';
import { type MaskLike } from '@abra/core/mask';
import { MixedControl, SliderControl } from '@/src/lib/project/edit-sections';

/**
 * The settings of the print from the values of every slider, in the order they are found: the look, the colors of the background, the
 * skin, the hair, the clothes and the accessories, the dots on the skin and on the background, the lines and the shadows.
 */
const optionsOf = ([
  [strength, colors, merge],
  [background],
  [skin],
  [hair],
  [clothes],
  [accessories],
  [dotColor, dotSpacing, dots],
  [backgroundDotColor, backgroundDotSpacing, backgroundDots],
  [outlines, lineWidth, smoothness, corners, lineColor],
  [shadows, shadowsStrength, shadowsColor],
]: number[][]): PopArtOptions => ({
  strength,
  colors,
  merge,
  background,
  backgroundDots,
  backgroundDotSpacing,
  backgroundDotColor,
  skin,
  hair,
  clothes,
  accessories,
  dots,
  dotSpacing,
  dotColor,
  outlines,
  lineWidth,
  smoothness,
  corners,
  lineColor,
  shadows,
  shadowsStrength,
  shadowsColor,
});

const defaults = POP_ART_DEFAULTS;
/** The most a setting that chooses a color can be: the last place in the list of colors. */
const LAST_COLOR = POP_ART_COLORS.length - 1;

/**
 * The named colors a setting chooses from, each setting the slider at `at` to its place in the list and the sliders before it to `before`.
 * They are the library's comic palette, read when they are first needed and not when this file loads, since the library is not ready then.
 */
const swatchesAt = (before: (number | undefined)[]) =>
  popArtSwatches().map(({ label, css }, choice) => ({ label, color: css, values: [...before, choice] }));

/**
 * A tab that chooses one color for a part of the print, only from the comic palette. A part that starts as `FROM_PHOTO` has no swatch
 * marked until one is chosen, and goes back to the photo's color when it is reset.
 */
const colorTab = (key: string, label: string, start: number): SliderControl<number> => ({
  kind: 'slider',
  key: `action-pop-art-${key}-color`,
  label,
  min: Math.min(0, start),
  max: LAST_COLOR,
  step: 1,
  defaultValue: start,
  picker: 'swatches',
  get swatches() {
    return swatchesAt([]);
  },
  applyOnSelect: true,
});

export const popArtControl: MixedControl = {
  kind: 'mixed',
  key: 'action-pop-art',
  label: 'Pop Art',
  thumb: (photo, { preloaded }) =>
    popArt(
      photo,
      POP_ART_DEFAULTS,
      preloaded && !Array.isArray(preloaded) ? (preloaded as Record<string, MaskLike>) : undefined,
    ),
  // The people are cut out once, in the background, with the parts of them when the models for it are on the device. Until then, and for a
  // photo with no people or no model, the whole picture is treated alike. The first slider carries it, as the effect is made from them all.
  tool: (photo, values, { preloaded }) =>
    popArt(
      photo,
      optionsOf(values),
      preloaded && !Array.isArray(preloaded) ? (preloaded as Record<string, MaskLike>) : undefined,
    ),
  controls: [
    {
      kind: 'slider',
      key: 'action-pop-art-look',
      label: 'Look',
      sliderLabel: ['Strength', 'Color Count', 'Combine Colors'],
      display: 'tabulated',
      min: [0, 2, 0],
      max: [1, 12, 1],
      step: [0.01, 1, 0.01],
      defaultValue: [defaults.strength, defaults.colors, defaults.merge],
      applyOnSelect: true,
      preload: subjectMask,
    } satisfies SliderControl<number>,
    colorTab('background', 'Background', defaults.background),
    colorTab('skin', 'Skin', defaults.skin),
    colorTab('hair', 'Hair', defaults.hair),
    colorTab('clothes', 'Clothes', defaults.clothes),
    colorTab('accessories', 'Accessories', defaults.accessories),
    {
      kind: 'mixed',
      key: 'action-pop-art-dots',
      label: 'Dots',
      controls: [
        {
          kind: 'slider',
          key: 'action-pop-art-skin-dots',
          label: 'Skin',
          sliderLabel: ['Color', 'Spacing', 'Dots'],
          display: 'tabulated',
          min: [Math.min(0, defaults.dotColor), 0, 0],
          max: [LAST_COLOR, 1, 1],
          step: [1, 0.01, 0.01],
          defaultValue: [defaults.dotColor, defaults.dotSpacing, defaults.dots],
          picker: ['swatches', undefined, undefined],
          get swatches() {
            return swatchesAt([]);
          },
          applyOnSelect: true,
        } satisfies SliderControl<number>,
        {
          kind: 'slider',
          key: 'action-pop-art-background-dots',
          label: 'Background',
          sliderLabel: ['Color', 'Spacing', 'Dots'],
          display: 'tabulated',
          min: [Math.min(0, defaults.backgroundDotColor), 0, 0],
          max: [LAST_COLOR, 1, 1],
          step: [1, 0.01, 0.01],
          defaultValue: [defaults.backgroundDotColor, defaults.backgroundDotSpacing, defaults.backgroundDots],
          picker: ['swatches', undefined, undefined],
          get swatches() {
            return swatchesAt([]);
          },
          applyOnSelect: true,
        } satisfies SliderControl<number>,
      ],
    },
    {
      kind: 'slider',
      key: 'action-pop-art-lines',
      label: 'Lines',
      sliderLabel: ['Darkness', 'Width', 'Smoothness', 'Corners', 'Color'],
      display: 'tabulated',
      min: [0, 0, 0, 0, 0],
      max: [1, 1, 1, 1, LAST_COLOR],
      step: [0.01, 0.01, 0.01, 0.01, 1],
      defaultValue: [defaults.outlines, defaults.lineWidth, defaults.smoothness, defaults.corners, defaults.lineColor],
      picker: [undefined, undefined, undefined, undefined, 'swatches'],
      get swatches() {
        return swatchesAt([undefined, undefined, undefined, undefined]);
      },
      applyOnSelect: true,
    } satisfies SliderControl<number>,
    {
      kind: 'slider',
      key: 'action-pop-art-shadows',
      label: 'Shadows',
      sliderLabel: ['Amount', 'Strength', 'Color'],
      display: 'tabulated',
      min: [0, 0, 0],
      max: [0.6, 1, LAST_COLOR],
      step: [0.01, 0.01, 1],
      defaultValue: [defaults.shadows, defaults.shadowsStrength, defaults.shadowsColor],
      picker: [undefined, undefined, 'swatches'],
      get swatches() {
        return swatchesAt([undefined, undefined]);
      },
      applyOnSelect: true,
    } satisfies SliderControl<number>,
  ],
};
