import { tanGradient } from '@/src/lib/color-bars';
import { owning } from '@/src/lib/faces';
import { paintSheet } from '@/src/lib/layers';
import { grayscale, invert, saturation } from '@abra/core/adjustments';
import { skinSmooth, skinTone } from '@abra/core/beauty';
import { blend, BlendMode } from '@abra/core/blend';
import { ResizeTarget } from '@abra/core/canvas';
import { gaussianBlur } from '@abra/core/filters';
import { TransformAlgorithm } from '@abra/core/mask';
import { resize } from '@abra/core/transform';
import { applyBlush } from '../blush';
import { applySculpt } from '../contour';
import { type EditSection } from '../edit-sections';
import { applyEyeMakeup, makeupMasks, skinWithMouthEased } from '../eye-makeup';
import { applyHairColor, HAIR_MIDDLE, HAIR_REACH } from '../hair-color';
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
  HAIR_COLOR_KEY,
  HAIR_GROUP_KEY,
  HIGHLIGHTER_KEY,
  LIP_LINER_KEY,
  LIPSTICK_KEY,
  MAKEUP_GROUP_KEY,
  RED_EYE_TOOL_KEY,
  SKIN_EVEN_OUT_KEY,
  SKIN_GROUP_KEY,
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

/** The natural hair colors, each once. How light the hair ends up is worked out from the color itself (see `hairLightness`). */
const hairColors = [
  { label: 'Black', color: '#1C1C1C' },
  { label: 'Dark brown', color: '#3B2418' },
  { label: 'Brown', color: '#5A3825' },
  { label: 'Chestnut', color: '#7A4A2B' },
  { label: 'Light brown', color: '#8E6A45' },
  { label: 'Auburn', color: '#8B3A1E' },
  { label: 'Ginger', color: '#B5532A' },
  { label: 'Copper', color: '#C06A3B' },
  { label: 'Dark blond', color: '#A8814F' },
  { label: 'Strawberry blond', color: '#D9A06B' },
  { label: 'Blond', color: '#E0C080' },
  { label: 'Platinum', color: '#EFE6C8' },
  { label: 'Gray', color: '#9A9A9A' },
  { label: 'Silver', color: '#C9C9CC' },
  { label: 'White', color: '#F4F4F4' },
];

/**
 * Where Lighten / Darken goes for a hair color (-1 to 1): the color's own brightness (HSV value), since the middle color of the hair is
 * the swatch's color. The slider's scale is the one `applyHairColor` reads.
 */
const hairLightness = (color: string) => {
  const [r, g, b] = [1, 3, 5].map(at => parseInt(color.slice(at, at + 2), 16));
  return Math.max(-1, Math.min(1, (Math.max(r, g, b) / 255 - HAIR_MIDDLE) / HAIR_REACH));
};

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

/**
 * How Even Out finds the skin's overall color. The color of the skin changes slowly, so it is worked out on a copy of the skin no
 * longer than this (in pixels) and scaled back up, which is the same colors for far less work. On that copy each pixel becomes the
 * average color of the skin within `EVEN_OUT_REACH` pixels of it, counting only the skin: the hair, the eyes, the lips and the
 * clothes next to it count for nothing, so they cannot dull or tint the skin's color. Past 1 on the slider the reach is multiplied by
 * the slider's value (as Smooth's blur is).
 */
const EVEN_OUT_SIZE = 160;
const EVEN_OUT_REACH = 16;

/** How fine the detail Texture puts back is, as a fraction of the longest side of the skin's crop. Pores, not blotches. */
const TEXTURE_SCALE = 0.0025;

export const beauty: EditSection = {
  key: 'section-beauty',
  label: 'Beauty',
  controls: [
    {
      kind: 'group',
      key: SKIN_GROUP_KEY,
      label: 'Skin',
      controls: [
        {
          kind: 'slider',
          key: SKIN_EVEN_OUT_KEY,
          label: 'Even Skin Tone',
          display: 'tabulated',
          sliderLabel: ['Even Skin Tone', 'Colorfulness'],
          // Even Out: up to 1 is how much of the evened color shows; past 1 the color is evened over a wider reach, up to 3.
          // Vividness adjusts the color after it has been evened, without changing the photo outside the skin mask.
          min: [0, -100],
          max: [3, 200],
          step: [0.1, 1],
          defaultValue: [0, 0],
          applyOnSelect: false,
          preload: makeupMasks,
          // A layer in the skin group, under the tan: the skin blurred into one smooth color, given back the photo's own light and shade
          // (see `paint`), so patches, tan lines and a face that does not match the neck fade while every pore stays.
          layer: {
            group: 'skin',
            blend: BlendMode.Normal,
            opacity: ([amount]) => Math.min(amount, 1),
            paint: (sheet, [amount, vividness]: number[], { skin, preloaded }) =>
              owning(keep => {
                const found = skin?.();
                if (!found) return;
                // This replaces the color of the skin, so the lips, the eyes and the eyebrows are taken out of it completely.
                const mask = keep(
                  skinWithMouthEased(keep(found), preloaded, keep, 1, ['lipstick|', 'mouth|', 'eye|', 'brow|']),
                );
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
                // How light each pixel of the photo is, kept to give back after the color is evened.
                const light = keep(image.clone());
                keep(grayscale()).apply(light);
                const here = keep(mask.cropped(part));
                const shrink = Math.min(1, EVEN_OUT_SIZE / Math.max(part.width, part.height));
                const small = {
                  width: Math.max(1, Math.round(part.width * shrink)),
                  height: Math.max(1, Math.round(part.height * shrink)),
                };
                keep(resize(ResizeTarget.Exact.new(small))).apply(image);
                // The average color of only the skin: the photo is multiplied by how much each pixel is skin, both are blurred, and the
                // blurred photo is divided by the blurred skin, which leaves out everything that is not skin.
                const skinHere = keep(
                  keep(here.resized(small.width, small.height, TransformAlgorithm.Bilinear)).toImage(),
                );
                keep(keep(blend(skinHere)).withMode(BlendMode.Multiply)).apply(image);
                const reach = EVEN_OUT_REACH * Math.max(1, amount);
                keep(gaussianBlur(reach)).apply(image);
                keep(gaussianBlur(reach)).apply(skinHere);
                keep(keep(blend(skinHere)).withMode(BlendMode.Divide)).apply(image);
                keep(resize(ResizeTarget.Exact.new({ width: part.width, height: part.height }))).apply(image);
                // Puts the photo's light back on the evened color. Color blend does this in HSL, where a bright pixel can hold less
                // color than a dark one, so the bright skin came out pale. This adds the same difference in light to red, green and
                // blue, which keeps the color's strength: how light the evened color is, turned upside down, laid at half strength
                // over how light the photo is, is gray plus half their difference, and Linear Light adds twice that to the color.
                const evened = keep(image.clone());
                keep(grayscale()).apply(evened);
                keep(invert()).apply(evened);
                keep(keep(blend(evened)).withOpacity(0.5)).apply(light);
                keep(keep(blend(light)).withMode(BlendMode.LinearLight)).apply(image);
                keep(saturation(vividness)).apply(image);
                here.applyToImage(image);
                sheet.painted = { image, x, y };
              }),
          },
        },
        {
          kind: 'slider',
          key: SKIN_TAN_KEY,
          label: 'Tan',
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
                paintSheet(
                  sheet,
                  [skinWithMouthEased(keep(found), preloaded, keep)],
                  keep(tanGradient().colorAt(offset)),
                );
              }),
          },
        },
        {
          kind: 'slider',
          key: SKIN_TONE_KEY,
          label: 'Lighten / Darken',
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
            overEarlier: true,
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
          key: SKIN_SMOOTH_KEY,
          label: 'Smooth',
          display: 'tabulated',
          sliderLabel: ['Smooth', 'Texture'],
          // Smooth: up to 1 is how much of the smoothing shows; past 1 the smoothing itself gets stronger, up to 3. Texture: how much
          // of the skin's fine detail, the pores, is put back over the smoothed skin. Projects saved before it have none.
          min: [0, 0],
          max: [3, 1],
          step: [0.1, 0.01],
          defaultValue: [0, 1],
          applyOnSelect: false,
          // A layer in the skin group: the part of the photo the skin reaches, smoothed, shown only where the skin is.
          layer: {
            group: 'skin',
            overEarlier: true,
            blend: BlendMode.Normal,
            opacity: () => 1,
            paint: (sheet, [amount, texture]: number[], { skin }) =>
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
                keep(keep(skinSmooth(amount)).withMask(here)).apply(image);
                if (amount > 0 && texture > 0) {
                  // The fine detail of the photo, as a retoucher's high pass: the photo laid with a slight blur of itself turned
                  // upside down at half strength, which is gray where the skin is smooth and lighter or darker at a pore. Linear
                  // Light adds that detail back over the smoothed skin, as much as `texture` says.
                  const detail = keep(sheet.photo.cropped(part.x, part.y, part.width, part.height));
                  const soft = keep(sheet.photo.cropped(part.x, part.y, part.width, part.height));
                  keep(gaussianBlur(Math.max(1, Math.max(part.width, part.height) * TEXTURE_SCALE))).apply(soft);
                  keep(invert()).apply(soft);
                  keep(keep(blend(soft)).withOpacity(0.5)).apply(detail);
                  keep(keep(keep(blend(detail)).withMode(BlendMode.LinearLight)).withOpacity(texture)).apply(image);
                }
                here.applyToImage(image);
                sheet.painted = { image, x, y };
              }),
          },
        },
      ],
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
          sliderLabel: ['Color', 'Strength', 'Colorfulness', 'Brightness', 'Size'],
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
          label: 'Brow Highlight',
          display: 'tabulated',
          sliderLabel: ['Color', 'Strength', 'Colorfulness', 'Brightness', 'Size'],
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
          label: 'Concealer',
          display: 'tabulated',
          sliderLabel: ['Color', 'Strength', 'Colorfulness', 'Brightness', 'Size'],
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
          sliderLabel: ['Color', 'Strength', 'Softness', 'Size', 'Colorfulness', 'Brightness'],
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
          sliderLabel: ['Color', 'Strength', 'Softness', 'Colorfulness', 'Brightness'],
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
          sliderLabel: ['Color', 'Strength', 'Width', 'Colorfulness', 'Brightness'],
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
          sliderLabel: ['Color', 'Strength', 'Softness', 'Size', 'Spread'],
          min: [0, 0, 0, 0.5, 0],
          max: [1, 1, 1, 1.5, 1],
          step: [0.01, 0.01, 0.01, 0.01, 0.01],
          // Opening the control adds a light glow to the cheekbones, nose, forehead and chin at once. Fan swings each cheek's glow in
          // towards the nose.
          defaultValue: [0, 0.2, 1, 1, 0],
          picker: ['glow', undefined, undefined, undefined, undefined],
          // Tints people ask for by name, each a place on the bar (the bar has a stop for each, evenly spaced).
          swatches: [
            { label: 'Pearl', color: 'rgb(255,250,245)', values: [0, undefined, undefined, undefined, undefined] },
            {
              label: 'Champagne',
              color: 'rgb(247,231,206)',
              values: [0.25, undefined, undefined, undefined, undefined],
            },
            { label: 'Gold', color: 'rgb(234,200,120)', values: [0.5, undefined, undefined, undefined, undefined] },
            { label: 'Peach', color: 'rgb(255,190,150)', values: [0.75, undefined, undefined, undefined, undefined] },
            { label: 'Rose', color: 'rgb(244,170,180)', values: [1, undefined, undefined, undefined, undefined] },
          ],
          // The AI finds the face once, when the control opens; dragging only changes how it looks.
          preload: makeupMasks,
          layer: {
            blend: BlendMode.Screen,
            opacity: ([, strength]) => strength,
            paint: (sheet, [tint, , softness, size, fan = 0]: number[], context) =>
              applySculpt('highlighter', sheet, softness, size, tint, context, fan),
          },
        },
        {
          kind: 'slider',
          key: CONTOUR_KEY,
          label: 'Face Shaping',
          // First the shade of the shadow (a bar from a light tan to a dark brown), then how much, how soft it is, how big and how much the hollow fans out towards the mouth.
          display: 'tabulated',
          sliderLabel: ['Shade', 'Strength', 'Softness', 'Size', 'Spread'],
          min: [0, 0, 0, 0.5, 0],
          max: [1, 1, 1, 1.5, 1],
          step: [0.01, 0.01, 0.01, 0.01, 0.01],
          // Opening the control adds a soft shade under the cheekbones and beside the nose at once.
          defaultValue: [0.5, 0.2, 1, 1.5, 0],
          picker: ['shade', undefined, undefined, undefined, undefined],
          // The AI finds the face once, when the control opens; dragging only changes how it looks.
          preload: makeupMasks,
          layer: {
            blend: BlendMode.Multiply,
            opacity: ([, strength]) => strength,
            paint: (sheet, [shade, , softness, size, fan = 0]: number[], context) =>
              applySculpt('contour', sheet, softness, size, shade, context, fan),
          },
        },
      ],
    },
    {
      kind: 'group',
      key: HAIR_GROUP_KEY,
      label: 'Hair',
      controls: [
        {
          kind: 'slider',
          key: HAIR_COLOR_KEY,
          label: 'Hair Color',
          // First the color (a swatch or the bar), then how much of it, and how colorful and how light or dark the hair comes out.
          display: 'tabulated',
          sliderLabel: ['Color', 'Strength', 'Colorfulness', 'Lighten / Darken'],
          min: [0, 0, 0, -1],
          max: [1, 1, 1, 1],
          step: [0.01, 0.01, 0.01, 0.01],
          // Opening the control gives the hair a natural chestnut at once, so there is something to change.
          defaultValue: [1 - 22 / 360, 1, 0.65, hairLightness('#7A4A2B')],
          picker: ['hue', undefined, undefined, undefined],
          // The natural colors; each also sets how light the hair comes out, from its own color.
          swatches: swatchesOf(hairColors, (hue, saturation, _brightness, label) => [
            hue,
            1,
            saturation,
            hairLightness(hairColors.find(each => each.label === label)!.color),
          ]),
          // The AI finds the hair once, when the control opens; dragging only changes how it looks.
          preload: makeupMasks,
          layer: {
            blend: BlendMode.Normal,
            opacity: ([, strength]) => strength,
            paint: (sheet, [hue, , saturation, shift]: number[], context) =>
              applyHairColor(sheet, hue, saturation, shift, context),
          },
        },
      ],
    },
  ],
};
