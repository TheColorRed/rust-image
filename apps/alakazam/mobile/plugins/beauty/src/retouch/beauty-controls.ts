import { grayscale, invert, saturation } from '@abra/core/adjustments';
import { skinSmooth, skinTone } from '@abra/core/beauty';
import { blend, BlendMode } from '@abra/core/blend';
import { ResizeTarget } from '@abra/core/canvas';
import { Color } from '@abra/core/color';
import { gaussianBlur } from '@abra/core/filters';
import { TransformAlgorithm } from '@abra/core/mask';
import { resize } from '@abra/core/transform';
import { RemoverShape, ShapePosition, Units, remover } from '@abra/core/tools';
import type { EditControl } from '@alakazam/sdk';
import { app, owning, paintSheet, release, swatchesOf, type Painting, type ToolContext } from '@alakazam/sdk';
import { applyHairColor, HAIR_MIDDLE, HAIR_REACH } from '../hair/hair-color';
import { hairMasks } from '../hair/hair-masks';
import { makeupMasks, skinWithMouthEased } from '../skin/skin-masks';
import { applyTeethWhitening } from './teeth';
import { applyRedEyeToEditor } from './red-eye-fix';

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

const TAN_STOPS = [
  [255, 255, 255],
  [210, 180, 140],
  [181, 101, 29],
  [139, 90, 43],
  [101, 67, 33],
  [0, 0, 0],
];
const tanColorAt = (position: number) => {
  const at = Math.max(0, Math.min(1, position)) * (TAN_STOPS.length - 1);
  const low = Math.floor(at);
  const high = Math.min(low + 1, TAN_STOPS.length - 1);
  const mix = at - low;
  const rgb = TAN_STOPS[low].map((channel, index) => Math.round(channel + (TAN_STOPS[high][index] - channel) * mix));
  return Color.fromRgb(rgb[0], rgb[1], rgb[2]);
};
const SKIN_GROUP_KEY = 'group-skin';
const HAIR_GROUP_KEY = 'group-hair';
const SKIN_EVEN_OUT_KEY = 'action-skin-even-out';
const SKIN_TAN_KEY = 'action-skin-tan';
const SKIN_TONE_KEY = 'action-skin-tone';
const SKIN_SMOOTH_KEY = 'action-skin-smooth';
const TEETH_WHITENING_KEY = 'slider-teeth-whitening';
const RED_EYE_TOOL_KEY = 'action-red-eye';
const BLEMISH_TOOL_KEY = 'action-blemish';
const HAIR_COLOR_KEY = 'slider-hair-color';
const HAIR_MODELS = ['skin-segmentation'];

export const beauty: { key: string; label: string; controls: EditControl[] } = {
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
          person: true,
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
          person: true,
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
                paintSheet(sheet, [skinWithMouthEased(keep(found), preloaded, keep)], keep(tanColorAt(offset)));
              }),
          },
        },
        {
          kind: 'slider',
          key: SKIN_TONE_KEY,
          label: 'Lighten / Darken',
          person: true,
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
          person: true,
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
      person: true,
      face: true,
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
    { kind: 'tool', key: RED_EYE_TOOL_KEY, label: 'Red Eye', onApply: applyRedEyeToEditor },
    {
      kind: 'tool',
      key: BLEMISH_TOOL_KEY,
      label: 'Blemish',
      onApply: value => {
        const target = value as { x: number; y: number; radius: number } | undefined;
        const image = app.editor.currentImage();
        if (!target || !image || !app.editor.hasHistory()) return;
        app.editor.flush();
        const photo = image.clone();
        try {
          remover(RemoverShape.Circle.new(target.radius))
            .withDistance(Units.Pixels.new(Math.max(Math.round(target.radius * 0.6), 3)))
            .withPosition(ShapePosition.Point.new({ x: target.x, y: target.y }))
            .apply(photo);
          app.editor.replaceImage(image, photo);
          app.history.commitRetouch({ type: 'blemish', ...target });
          void app.history.save();
        } finally {
          release(photo);
        }
      },
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
          person: true,
          needsModels: { title: 'Hair Color', roles: HAIR_MODELS },
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
          preload: hairMasks,
          analysisGroup: 'hair',
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

export const beautyControl = (key: string): EditControl | undefined => {
  const find = (controls: EditControl[]): EditControl | undefined => {
    for (const control of controls) {
      if (control.key === key) return control;
      if ('controls' in control) {
        const nested = find(control.controls);
        if (nested) return nested;
      }
    }
    return undefined;
  };
  return find(beauty.controls);
};



