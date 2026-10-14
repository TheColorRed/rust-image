import { tanGradient } from '@/src/lib/color-bars';
import { owning } from '@/src/lib/faces';
import { paintSheet } from '@/src/lib/layers';
import { skinTone } from '@abra/core/beauty';
import { BlendMode } from '@abra/core/blend';
import { applyEffect, EffectSpec } from '@alakazam/mobile';
import { applyBlush } from '../blush';
import { applySculpt } from '../contour';
import { type EditSection } from '../edit-sections';
import { applyEyeMakeup, makeupMasks, skinWithMouthEased } from '../eye-makeup';
import { applyLipLiner } from '../lip-liner';
import { applyLipstick } from '../lipstick';
import { applyTeethWhitening } from '../teeth';
import {
  BLEMISH_TOOL_KEY,
  BLUSH_KEY,
  BROW_BONE_KEY,
  CONTOUR_KEY,
  EYELINER_KEY,
  EYESHADOW_KEY,
  HIGHLIGHTER_KEY,
  LIP_LINER_KEY,
  LIPSTICK_KEY,
  MAKEUP_GROUP_KEY,
  RED_EYE_TOOL_KEY,
  SKIN_SMOOTH_KEY,
  SKIN_TAN_KEY,
  SKIN_TONE_KEY,
  TEETH_WHITENING_KEY,
  UNDER_EYE_KEY,
} from './keys';

/** The colors the lips offer, each once. `lipSwatches` below turns them into the lipstick and lip liner swatches. */
const lipColors = [
  { label: 'Black', color: '#000000' },
  { label: 'White', color: '#FFFFFF' },
  { label: 'Creamy nude', color: '#EBAD81' },
  { label: 'Nude peach', color: '#F18E6A' },
  { label: 'Nude', color: '#DD7C3D' },
  { label: 'Caramel nude', color: '#B87A74' },
  { label: 'Dark nude', color: '#D27754' },
  { label: 'Coffee nude', color: '#B46F4B' },
  { label: 'Pale pink', color: '#F096BD' },
  { label: 'Nude pink', color: '#E9A8C6' },
  { label: 'Pale rose pink', color: '#EC7EAA' },
  { label: 'Rose pink', color: '#DF4A86' },
  { label: 'Browny pink', color: '#C06D7E' },
  { label: 'Dark dusky pink', color: '#AE5373' },
  { label: 'Dark pink', color: '#BC316A' },
  { label: 'Deep rose pink', color: '#E32957' },
  { label: 'Bright fuchsia', color: '#E30274' },
  { label: 'Sheer coral', color: '#EE8372' },
  { label: 'Sheer strawberry pink', color: '#E74957' },
  { label: 'Coral red', color: '#E73C42' },
  { label: 'Deep mandarin', color: '#E6421A' },
  { label: 'Bright red', color: '#E0011B' },
  { label: 'Classic red', color: '#CD0215' },
  { label: 'Deep red', color: '#BF012F' },
  { label: 'Sheer plum', color: '#C58396' },
  { label: 'Golden plum', color: '#9E4776' },
  { label: 'Plum', color: '#A6315B' },
  { label: 'Raspberry pink', color: '#B00048' },
  { label: 'Burgundy', color: '#7F023E' },
  { label: 'Deep purple plum', color: '#8C0364' },
];

/** The colors the face makeup offers, each once. A control turns them into its own swatches with `swatchesOf`. */
const faceColors = [
  { label: 'Black', color: '#111111' },
  { label: 'Brown', color: '#553126' },
  { label: 'Navy', color: '#202C50' },
  { label: 'Pearl', color: '#F2E8DC' },
  { label: 'Brighten', color: '#EFD9CA' },
  { label: 'Champagne', color: '#D8B68A' },
  { label: 'Soft gold', color: '#D3AD68' },
  { label: 'Light peach', color: '#EDB79F' },
  { label: 'Peach', color: '#E89A78' },
  { label: 'Orange', color: '#D9784D' },
  { label: 'Soft pink', color: '#E8A0A8' },
  { label: 'Pink', color: '#DEA1AB' },
  { label: 'Rose', color: '#B97882' },
  { label: 'Rose glow', color: '#D6A5A3' },
  { label: 'Coral', color: '#EE7A66' },
  { label: 'Berry', color: '#B04A6A' },
  { label: 'Plum', color: '#70445F' },
  { label: 'Bronze', color: '#A66B3F' },
  { label: 'Bronze glow', color: '#AA754D' },
  { label: 'Smoky', color: '#55515A' },
];

/**
 * Makes a control's swatches from a list of colors. `values` lays the color out the way the control's sliders do, from the
 * color's hue (turned the way the color bar runs), vividness and brightness.
 */
const swatchesOf = (
  colors: { label: string; color: string }[],
  values: (hue: number, saturation: number, brightness: number, label: string) => (number | undefined)[],
) =>
  colors.map(({ label, color }) => {
    const [r, g, b] = [1, 3, 5].map(at => parseInt(color.slice(at, at + 2), 16) / 255);
    const high = Math.max(r, g, b);
    const range = high - Math.min(r, g, b);
    const degrees = !range
      ? 0
      : high === r
        ? ((g - b) / range + 6) % 6
        : high === g
          ? (b - r) / range + 2
          : (r - g) / range + 4;
    return { label, color, values: values((1 - (degrees * 60) / 360) % 1, high ? range / high : 0, high, label) };
  });

/** The lipstick and lip liner swatches. The sheer ones are lighter. */
const lipSwatches = swatchesOf(lipColors, (hue, saturation, brightness, label) => [
  hue,
  label.startsWith('Sheer') ? 0.45 : undefined,
  undefined,
  saturation,
  brightness,
]);

/**
 * How lip color is laid over the lips: with the lips' own light and shade showing through (Color), except for black, which darkens
 * them (Multiply), and white, which covers them (Normal). The values are the lipstick's and the lip liner's: the vividness and the
 * brightness are the fourth and the fifth.
 */
const lipBlend = ([, , , saturation, brightness]: number[]) =>
  brightness < 0.2 ? BlendMode.Multiply : saturation < 0.1 && brightness > 0.9 ? BlendMode.Normal : BlendMode.Color;

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
      // A layer in the skin group: the part of the photo the skin reaches, smoothed, shown only where the skin is.
      layer: {
        group: 'skin',
        blend: BlendMode.Normal,
        opacity: () => 1,
        paint: (sheet, [amount]: number[], { skin }) =>
          owning(keep => {
            const found = skin?.();
            if (!found) return;
            const mask = keep(found);
            const bounds = mask.bounds(0);
            if (!bounds) return;
            const [x, y] = [Math.floor(bounds.x), Math.floor(bounds.y)];
            const part = {
              x,
              y,
              width: Math.min(sheet.width(), Math.ceil(bounds.x + bounds.width)) - x,
              height: Math.min(sheet.height(), Math.ceil(bounds.y + bounds.height)) - y,
            };
            if (part.width < 1 || part.height < 1) return;
            const image = sheet.photo.cropped(part.x, part.y, part.width, part.height);
            const here = keep(mask.cropped(part));
            applyEffect(image, EffectSpec.SkinSmooth.new({ amount }), here);
            here.applyToImage(image);
            sheet.painted = { image, x, y };
          }),
      },
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
      preload: makeupMasks,
      // A layer in the skin group: the color at this place on the tan scale, multiplied over the skin the editor found, except the
      // lips and the mouth.
      layer: {
        group: 'skin',
        blend: BlendMode.Multiply,
        opacity: () => 1,
        paint: (sheet, [offset]: number[], { skin, preloaded }) =>
          owning(keep => {
            const found = skin?.();
            if (!found) return;
            paintSheet(sheet, [skinWithMouthEased(keep(found), preloaded, keep)], keep(tanGradient().colorAt(offset)));
          }),
      },
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
      preload: makeupMasks,
      // A layer in the skin group: the part of the photo the skin reaches, lightened or darkened, shown only where the skin is
      // (the lips and the mouth eased), so it replaces the photo there.
      layer: {
        group: 'skin',
        blend: BlendMode.Normal,
        opacity: () => 1,
        paint: (sheet, [amount]: number[], { skin, preloaded }) =>
          owning(keep => {
            const found = skin?.();
            if (!found) return;
            const mask = keep(skinWithMouthEased(keep(found), preloaded, keep));
            const bounds = mask.bounds(0);
            if (!bounds) return;
            const [x, y] = [Math.floor(bounds.x), Math.floor(bounds.y)];
            const part = {
              x,
              y,
              width: Math.min(sheet.width(), Math.ceil(bounds.x + bounds.width)) - x,
              height: Math.min(sheet.height(), Math.ceil(bounds.y + bounds.height)) - y,
            };
            if (part.width < 1 || part.height < 1) return;
            const image = sheet.photo.cropped(part.x, part.y, part.width, part.height);
            const here = keep(mask.cropped(part));
            keep(keep(skinTone(amount)).withMask(here)).apply(image);
            here.applyToImage(image);
            sheet.painted = { image, x, y };
          }),
      },
    },
    {
      kind: 'slider',
      key: TEETH_WHITENING_KEY,
      label: 'Teeth Whitening',
      display: 'tabulated',
      sliderLabel: ['Whiten', 'Brighten'],
      min: 0,
      max: 100,
      step: 0.01,
      defaultValue: 0,
      applyOnSelect: false,
      // The AI finds the mouth once, when the control opens; the teeth themselves are found by their color.
      preload: makeupMasks,
      // A layer in the skin group: the part of the photo around the mouth with its color taken out and made lighter, shown only on the teeth.
      layer: {
        group: 'skin',
        blend: BlendMode.Normal,
        opacity: () => 1,
        paint: (sheet, [whiten, brighten]: number[], context) => applyTeethWhitening(sheet, whiten, brighten, context),
      },
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
          sliderLabel: ['Color', 'Strength', 'Width'],
          min: [0, 0, 1.5],
          max: [1, 1, 3],
          step: [0.01, 0.01, 0.01],
          defaultValue: [0, 0.8, 1.5],
          picker: ['hue', undefined, undefined],
          swatches: swatchesOf(faceColors, hue => [hue]),
          preload: makeupMasks,
          layer: {
            blend: BlendMode.Multiply,
            opacity: ([, strength]) => strength,
            // Projects saved before the width slider have no third value; they get the thinnest width.
            paint: (sheet, [hue, , width = 1.5]: number[], context) =>
              applyEyeMakeup('eyeliner', sheet, hue, 0, 0.07, width, context),
          },
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
          defaultValue: [1 - 24 / 360, 0.8, 0.8, 0.8, 1],
          picker: ['hue', undefined, undefined, undefined, undefined],
          swatches: swatchesOf(faceColors, (hue, saturation, brightness) => [hue, undefined, saturation, brightness]),
          preload: makeupMasks,
          layer: {
            blend: BlendMode.Color,
            opacity: ([, strength]) => strength,
            paint: (sheet, [hue, , saturation, brightness, fullness]: number[], context) =>
              applyEyeMakeup('eyeshadow', sheet, hue, saturation, brightness, fullness, context),
          },
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
          defaultValue: [1 - 38 / 360, 0.8, 0.8, 0.8, 1],
          picker: ['hue', undefined, undefined, undefined, undefined],
          swatches: swatchesOf(faceColors, (hue, saturation, brightness) => [hue, undefined, saturation, brightness]),
          preload: makeupMasks,
          layer: {
            blend: BlendMode.SoftLight,
            opacity: ([, strength]) => strength,
            paint: (sheet, [hue, , saturation, brightness, fullness]: number[], context) =>
              applyEyeMakeup('brow-bone', sheet, hue, saturation, brightness, fullness, context),
          },
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
          defaultValue: [1 - 24 / 360, 0.8, 0.8, 0.8, 1],
          picker: ['hue', undefined, undefined, undefined, undefined],
          swatches: swatchesOf(faceColors, (hue, saturation, brightness) => [hue, undefined, saturation, brightness]),
          preload: makeupMasks,
          layer: {
            blend: BlendMode.Color,
            opacity: ([, strength]) => strength,
            paint: (sheet, [hue, , saturation, brightness, fullness]: number[], context) =>
              applyEyeMakeup('under-eye', sheet, hue, saturation, brightness, fullness, context),
          },
        },
        {
          kind: 'slider',
          key: BLUSH_KEY,
          label: 'Blush',
          // First the color, then how much, how soft it is and how wide, and for a custom color how vivid and bright.
          display: 'tabulated',
          sliderLabel: ['Color', 'Strength', 'Softness', 'Size', 'Vividness', 'Brightness'],
          min: [0, 0, 0, 0.5, 0, 0],
          max: [1, 1, 1, 1.5, 1, 1],
          step: [0.01, 0.01, 0.01, 0.01, 0.01, 0.01],
          // Opening the control puts a soft pink on the cheeks at once, so there is something to change.
          defaultValue: [1 - 350 / 360, 0.8, 0.8, 1, 0.8, 0.8],
          picker: ['hue', undefined, undefined, undefined, undefined, undefined],
          swatches: swatchesOf(faceColors, (hue, saturation, brightness) => [
            hue,
            undefined,
            undefined,
            undefined,
            saturation,
            brightness,
          ]),
          // The AI finds the face once, when the control opens; dragging only changes how the blush looks.
          preload: makeupMasks,
          layer: {
            blend: BlendMode.Color,
            opacity: ([, strength]) => strength,
            paint: (sheet, [hue, , softness, size, saturation, brightness]: number[], context) =>
              applyBlush(sheet, hue, softness, size, saturation, brightness, context),
          },
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
          defaultValue: [0, 0.8, 1, 0.8, 0.8],
          picker: ['hue', undefined, undefined, undefined, undefined],
          // Colors people ask for by name, from a shade chart of real lipsticks (each color is read from the chart; its name is the
          // plain description of it). Shades that were nearly the same color (a color distance under about 8) are left out, so every
          // swatch is a choice a person can see. A swatch is a hue on the bar (1 minus its degrees around the wheel over 360, since the bar runs
          // backwards), how vivid it is and how bright. A "sheer" shade also sets a light strength. Black and white come first.
          swatches: lipSwatches,
          // The AI finds the lips once, when the control opens; dragging only changes the color, the strength or the edge.
          preload: makeupMasks,
          layer: {
            blend: lipBlend,
            opacity: ([, amount]) => amount,
            paint: (sheet, [hue, , softness, saturation, brightness]: number[], context) =>
              applyLipstick(sheet, hue, softness, saturation, brightness, context),
          },
        },
        {
          kind: 'slider',
          key: LIP_LINER_KEY,
          label: 'Lip Liner',
          display: 'tabulated',
          sliderLabel: ['Color', 'Strength', 'Width', 'Vividness', 'Brightness'],
          min: [0, 0, 0, 0, 0],
          max: [1, 1, 1, 1, 1],
          step: [0.01, 0.01, 0.01, 0.01, 0.01],
          // Opening the control draws a soft rosy-brown line at once, so there is something to change.
          defaultValue: [1 - 358 / 360, 0.8, 0.4, 0.8, 0.8],
          picker: ['hue', undefined, undefined, undefined, undefined],
          swatches: lipSwatches,
          // The AI finds the lips once, when the control opens; dragging only changes the line.
          preload: makeupMasks,
          layer: {
            blend: lipBlend,
            opacity: ([, strength]) => strength,
            paint: (sheet, [hue, , width, saturation, brightness]: number[], context) =>
              applyLipLiner(sheet, hue, width, saturation, brightness, context),
          },
        },
        {
          kind: 'slider',
          key: HIGHLIGHTER_KEY,
          label: 'Highlighter',
          display: 'tabulated',
          sliderLabel: ['Strength', 'Softness', 'Size'],
          min: [0, 0, 0.5],
          max: [1, 1, 1.5],
          step: [0.01, 0.01, 0.01],
          // Opening the control adds a light glow to the cheekbones, nose, forehead and chin at once.
          defaultValue: [0.8, 0.7, 1],
          // The AI finds the face once, when the control opens; dragging only changes how it looks.
          preload: makeupMasks,
          layer: {
            blend: BlendMode.Screen,
            opacity: ([strength]) => strength,
            paint: (sheet, [, softness, size]: number[], context) =>
              applySculpt('highlighter', sheet, softness, size, 0, context),
          },
        },
        {
          kind: 'slider',
          key: CONTOUR_KEY,
          label: 'Contour',
          // First the shade of the shadow (a bar from a light tan to a dark brown), then how much, how soft it is and how big.
          display: 'tabulated',
          sliderLabel: ['Shade', 'Strength', 'Softness', 'Size'],
          min: [0, 0, 0, 0.5],
          max: [1, 1, 1, 1.5],
          step: [0.01, 0.01, 0.01, 0.01],
          // Opening the control adds a soft shade under the cheekbones and beside the nose at once.
          defaultValue: [0.5, 0.8, 0.8, 1],
          picker: ['shade', undefined, undefined, undefined],
          // The AI finds the face once, when the control opens; dragging only changes how it looks.
          preload: makeupMasks,
          layer: {
            blend: BlendMode.Multiply,
            opacity: ([, strength]) => strength,
            paint: (sheet, [shade, , softness, size]: number[], context) =>
              applySculpt('contour', sheet, softness, size, shade, context),
          },
        },
      ],
    },
  ],
};
