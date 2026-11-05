import { LinearGradientEffect, quantize } from '@abra/core/adjustments';
import { BlendMode } from '@abra/core/blend';
import { Color, ColorSpace, Harmony, HueDirection, type ColorLike } from '@abra/core/color';
import { ColorStop, Gradient } from '@abra/core/gradient';
import { ImageLike } from '@abra/core/image';
import { stopsOf } from '@/src/lib/color/gradient-math';
import { derivedFor } from '@/src/lib/editor/editor-preloads';
import type { Preloaded } from '@/src/lib/project/edit-sections';
import type { MaskLike } from '@abra/core/mask';
import { owning } from '@/src/lib/native/native-handles';
import type { GradientValue } from '@/src/state/gradient-picker';

/**
 * Where each setting of the gradient overlay is in the values the control saves: the strength is the control's one slider, and the
 * rest are the choices its pickers save and the places of the two handles on the photo, all kept after the slider (see `sliderValues`).
 */
export const GRADIENT_AT = { strength: 0, look: 1, blend: 2, startX: 3, startY: 4, endX: 5, endY: 6, area: 7, stops: 8 } as const;

/** Where the gradient goes, by the number the area picker saves: all of the photo, only what is behind the people in it, or only the people. */
export const GRADIENT_AREA = { whole: 0, background: 1, people: 2 } as const;

/**
 * The look that means the person made the gradient themselves. Its stops are saved after the handles (see `encodeStops`), and it is -1 and
 * not a place in `GRADIENT_LOOKS`, so adding a ready-made look later never changes what a saved gradient is.
 */
export const GRADIENT_CUSTOM = -1;

/** Where the handles start: down the middle of the photo, from near the top to near the bottom, as shares of its width and height. */
export const GRADIENT_HANDLES = [0.5, 0.15, 0.5, 0.85] as const;

/** How strong the gradient is until the slider is moved. */
export const GRADIENT_STRENGTH = 0.85;

/**
 * Every setting of the gradient from the control's saved values, with what is missing filled in: the sliders and pickers save only what
 * was changed, so a gradient that was never moved has no handles saved. The result is in the places `GRADIENT_AT` names.
 */
export const gradientValues = (values: readonly (number | undefined)[]): number[] => [
  values[GRADIENT_AT.strength] ?? GRADIENT_STRENGTH,
  values[GRADIENT_AT.look] ?? 0,
  values[GRADIENT_AT.blend] ?? 0,
  ...GRADIENT_HANDLES.map((fallback, index) => values[GRADIENT_AT.startX + index] ?? fallback),
  values[GRADIENT_AT.area] ?? GRADIENT_AREA.whole,
  // The stops of a gradient the person made, when there are any.
  ...values.slice(GRADIENT_AT.stops).map(value => value ?? 0),
];

/**
 * A gradient the person made as the numbers that are saved after the handles: how many color stops there are, then each as its place
 * and red, green and blue; then how many strength stops, then each as its place and strength. Neither list is empty.
 */
export const encodeStops = (gradient: GradientValue): number[] => [
  gradient.colors.length,
  ...gradient.colors.flatMap(stop => [stop.at, ...stop.color]),
  gradient.alphas.length,
  ...gradient.alphas.flatMap(stop => [stop.at, stop.alpha]),
];

/** The gradient the person made from the saved values, or `undefined` when there is none (or what is saved is not a gradient). */
export const decodeStops = (values: readonly number[]): GradientValue | undefined => {
  const tail = values.slice(GRADIENT_AT.stops);
  const colorCount = tail[0] ?? 0;
  const alphaCountAt = 1 + colorCount * 4;
  const alphaCount = tail[alphaCountAt] ?? 0;
  if (colorCount < 1 || alphaCount < 1 || tail.length < alphaCountAt + 1 + alphaCount * 2) return undefined;
  return {
    colors: Array.from({ length: colorCount }, (_, index) => {
      const at = 1 + index * 4;
      return { at: tail[at], color: [tail[at + 1], tail[at + 2], tail[at + 3]] as [number, number, number] };
    }),
    alphas: Array.from({ length: alphaCount }, (_, index) => {
      const at = alphaCountAt + 1 + index * 2;
      return { at: tail[at], alpha: tail[at + 1] };
    }),
  };
};

/** The saved values with the look set to a gradient the person made, which replaces any made before. */
export const withCustomGradient = (values: readonly (number | undefined)[], gradient: GradientValue): number[] => [
  ...gradientValues(values).slice(0, GRADIENT_AT.stops).map((value, index) => (index === GRADIENT_AT.look ? GRADIENT_CUSTOM : value)),
  ...encodeStops(gradient),
];

/** A color stop: where it is along the gradient (0 to 1), then red, green, blue and alpha (0 to 255). */
type Stop = readonly [at: number, red: number, green: number, blue: number, alpha: number];

/** The ready-made gradients people choose from, by the number their picker saves. */
export const GRADIENT_LOOKS: readonly { label: string; stops: readonly Stop[] }[] = [
  { label: 'Sunset', stops: [[0, 255, 94, 98, 255], [0.5, 255, 153, 102, 255], [1, 255, 206, 120, 255]] },
  { label: 'Ocean', stops: [[0, 0, 119, 182, 255], [0.5, 0, 180, 216, 255], [1, 144, 224, 239, 255]] },
  { label: 'Dusk', stops: [[0, 58, 12, 163, 255], [0.5, 156, 39, 176, 255], [1, 255, 128, 171, 255]] },
  { label: 'Forest', stops: [[0, 20, 83, 45, 255], [0.5, 56, 142, 60, 255], [1, 200, 230, 130, 255]] },
  { label: 'Rose', stops: [[0, 255, 175, 189, 255], [1, 255, 195, 160, 255]] },
  { label: 'Fade to black', stops: [[0, 0, 0, 0, 0], [1, 0, 0, 0, 255]] },
  { label: 'Fade to white', stops: [[0, 255, 255, 255, 0], [1, 255, 255, 255, 255]] },
  { label: 'Rainbow', stops: [[0, 255, 59, 48, 255], [0.2, 255, 204, 0, 255], [0.4, 76, 217, 100, 255], [0.6, 90, 200, 250, 255], [0.8, 88, 86, 214, 255], [1, 255, 45, 85, 255]] },
];

/**
 * The ways the photo's accent color can be turned into a gradient, by the key the sub-menu of the From photo card uses. Each is one of the
 * library's color harmonies: the colors it gives are the stops, spread evenly. Named in plain words for people who do not know color theory.
 */
export const PHOTO_HARMONIES = [
  { key: 'opposite', label: 'Opposite', harmony: () => Harmony.Complementary.new() },
  // The accent with the two colors either side of its opposite. The library gives them in the order of a color theme, so `order` puts them in
  // the order of a gradient (the same order `Gradient::harmony` uses): the darker accent, the accent, the two sides, the darker side.
  { key: 'soft-opposite', label: 'Soft opposite', harmony: () => Harmony.SplitComplementary.new(), order: [3, 0, 2, 1, 4] },
  { key: 'neighbors', label: 'Neighbors', harmony: () => Harmony.Analogous.new() },
  { key: 'neighbor-and-opposite', label: 'Mixed', harmony: () => Harmony.Compound.new() },
  { key: 'three-way', label: 'Three-way', harmony: () => Harmony.Triadic.new() },
  { key: 'four-way', label: 'Four-way', harmony: () => Harmony.Square.new() },
  { key: 'one-color', label: 'One color', harmony: () => Harmony.Monochromatic.new(5) },
  { key: 'darker', label: 'Fade to dark', harmony: () => Harmony.Shades.new(5) },
] as const satisfies readonly { key: string; label: string; harmony: () => Harmony; order?: readonly number[] }[];

export type PhotoHarmony = (typeof PHOTO_HARMONIES)[number]['key'];

/** How many colors are looked for in a photo, at most, to find its most colorful one. */
const PHOTO_COLORS = 4;
/** How different two colors of a photo may look (the distance between them in Lab, where about 2 is the least a person can see) and still be one. */
const MERGE_DISTANCE = 12;

/**
 * A gradient made from the photo's most colorful color by a color harmony (the color opposite it on the color wheel, its neighbors, and so
 * on; see `PHOTO_HARMONIES`), or `undefined` when the photo has no colors to find. The color is the most colorful of the photo's main colors, not the biggest: most photos are mostly one dull
 * color (sand, sky, a wall) and the color a person notices in one is its accent (a swimsuit, a flower). The library finds the main colors
 * in one call, from about 20000 pixels of the photo spread over all of it (never a smaller copy), and gives the harmony, so this costs the
 * same for a small photo as for a large one.
 */
export const gradientFromPhoto = (photo: ImageLike, scheme: PhotoHarmony = 'opposite'): GradientValue | undefined =>
  owning(keep => {
    // How colorful a color is: how much color it has (saturation) times how much light, so a dark color with a trace of hue does not win.
    const vividness = (color: ColorLike) => {
      const [, saturation, value] = color.hsv();
      return saturation * value;
    };
    const found = keep(keep(quantize(PHOTO_COLORS)).withTolerance(MERGE_DISTANCE))
      .palette(photo)
      .map(color => keep(color));
    const accent = found.reduce<ColorLike | undefined>((best, color) => (!best || vividness(color) > vividness(best) ? color : best), undefined);
    if (!accent) return undefined;
    const chosen: { harmony: () => Harmony; order?: readonly number[] } = PHOTO_HARMONIES.find(each => each.key === scheme)!;
    const made = accent.harmony(chosen.harmony()).map(color => keep(color));
    const scheme_colors = chosen.order ? chosen.order.map(index => made[index]) : made;
    return {
      colors: scheme_colors.map((color, index) => ({
        at: index / (scheme_colors.length - 1),
        color: Array.from(new Uint8Array(color.rgb())) as [number, number, number],
      })),
      alphas: [
        { at: 0, alpha: 255 },
        { at: 1, alpha: 255 },
      ],
    };
  });

/** A ready-made look as a gradient the dialog can edit, so a person can start from a look they like and change it. */
export const lookAsGradient = (index: number): GradientValue => {
  const stops = (GRADIENT_LOOKS[index] ?? GRADIENT_LOOKS[0]).stops;
  return {
    colors: stops.map(([at, red, green, blue]) => ({ at, color: [red, green, blue] as [number, number, number] })),
    alphas: stops.map(([at, , , , alpha]) => ({ at, alpha })),
  };
};

/**
 * The gradient the saved values show: the one the person made when the look is `GRADIENT_CUSTOM` (the first look if what is saved is
 * not a gradient), or else the ready-made look.
 */
export const gradientShown = (values: readonly (number | undefined)[]): GradientValue => {
  const all = gradientValues(values);
  return (all[GRADIENT_AT.look] === GRADIENT_CUSTOM ? decodeStops(all) : undefined) ?? lookAsGradient(all[GRADIENT_AT.look]);
};

/** How the gradient lies on the photo, in plain words, by the number their picker saves. */
export const GRADIENT_BLENDS: readonly { label: string; mode: BlendMode }[] = [
  { label: 'Cover', mode: BlendMode.Normal },
  { label: 'Soft', mode: BlendMode.SoftLight },
  { label: 'Tint', mode: BlendMode.Color },
  { label: 'Boost', mode: BlendMode.Overlay },
  { label: 'Darken', mode: BlendMode.Multiply },
  { label: 'Brighten', mode: BlendMode.Screen },
];

/**
 * Lays a gradient over `photo` in place, from the control's saved values (see `GRADIENT_AT`). The photo remembers the effect and the
 * GPU draws it, so this does no image work. The handles are shares of the photo's width and height, so the same values give the same
 * gradient on a screen-size copy and on the full-size photo.
 *
 * A gradient that goes only behind the people, or only on them, is limited to the cutout of the people that `subjectMask` finds, which
 * is in `preloaded` once it has been found. Until it is (or when there are no people to find) nothing is laid, so the whole photo is
 * never shown with a gradient meant for part of it. The library applies an effect limited to a mask at once and not later, so this costs
 * more than a gradient on the whole photo.
 */
export const gradientOverlay = (photo: ImageLike, values: number[], preloaded?: Preloaded): void =>
  owning(keep => {
    const [strength, , blendAt, startX, startY, endX, endY, area] = gradientValues(values);
    if (strength <= 0) return;
    const cutout = preloaded && !Array.isArray(preloaded) ? (preloaded.subject as MaskLike | undefined) : undefined;
    let limit: MaskLike | undefined;
    if (area !== GRADIENT_AREA.whole) {
      if (!cutout || !preloaded) return;
      // The background is the cutout turned over. It is made once for the cutout and kept, since a drag asks again for every move.
      limit =
        area === GRADIENT_AREA.background
          ? (derivedFor(preloaded, 'gradient-background', 'inverted', () => cutout.inverted()) as MaskLike | undefined)
          : cutout;
    }
    const blend = GRADIENT_BLENDS[blendAt] ?? GRADIENT_BLENDS[0];
    // Each place along the gradient where either its color or its strength has a stop is one stop of the library's gradient. The colors
    // between two stops are mixed in Oklab, which follows what the eye sees, so two colors far apart (an orange and a blue) pass through a
    // clean color and not the dull gray that mixing the red, green and blue values gives.
    const stops = stopsOf(gradientShown(values)).map(({ at, rgb, alpha }) =>
      keep(new ColorStop(keep(Color.fromRgba(rgb[0], rgb[1], rgb[2], alpha)), at)),
    );
    const gradient = keep(keep(new Gradient(stops)).withInterpolation({ space: ColorSpace.Oklab, hue: HueDirection.Shorter }));
    const width = photo.width();
    const height = photo.height();
    const effect = keep(
      keep(keep(LinearGradientEffect.between(gradient, [startX * width, startY * height], [endX * width, endY * height])).withOpacity(strength)).withBlendMode(
        blend.mode,
      ),
    );
    (limit ? keep(effect.withMask(limit)) : effect).applyDeferred(photo);
  });
