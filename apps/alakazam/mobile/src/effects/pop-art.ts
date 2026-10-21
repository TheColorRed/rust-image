import { highlightsShadows } from '@/src/effects/highlights-shadows';
import { owning } from '@/src/lib/native/native-handles';
import { autoTone, contrast, grayscale, quantize } from '@abra/core/adjustments';
import { blend, BlendMode } from '@abra/core/blend';
import { Color, ColorStat, Palette, type ColorLike, type PaletteLike } from '@abra/core/color';
import { Fill } from '@abra/core/fill';
import { colorEdges, halftone, median, surfaceBlur } from '@abra/core/filters';
import { Image } from '@abra/core/image';
import { Mask, type MaskLike } from '@abra/core/mask';
import { type AbraImageLike } from '@alakazam/mobile';

/** The size of the edge-keeping smoothing, as a share of the photo's shorter side, so the flat areas scale with the photo. */
const SMOOTHING = 0.008;
/** How different two neighbouring colors may be and still be smoothed together. Edges bigger than this are kept. */
const SMOOTH_LIMIT = 30;
/** The size of the median clean-up that takes the last specks out, as a share of the photo's shorter side. */
const CLEAN = 0.003;
/** How far apart two colors may be, at full `merge`, and still be merged into one. See `Quantize::with_tolerance` in the library. */
const MAX_TOLERANCE = 60;
/** How much the lightness counts, against the hue, when two colors are merged. */
const LIGHTNESS_WEIGHT = 0.6;
/** The thinnest and thickest the outlines can be, as a share of the photo's shorter side: the `lineWidth` setting goes from one to the other. */
const THINNEST_LINE = 0.002;
const THICKEST_LINE = 0.012;
/** How far a traced outline may be moved to drop the points it does not need, as a share of the people's size, at full `smoothness`. */
const MOST_TRACE_TOLERANCE = 0.008;
/** The sharpest turn, in degrees, that `corners` can ask to be kept as a corner: the setting goes from none (0) to this. */
const MOST_CORNER_DEGREES = 120;
/** The smallest and largest distance between dots, as a share of the size of what they are on: the `dotSpacing` setting goes from one to the other. */
const CLOSEST_DOTS = 0.015;
const WIDEST_DOTS = 0.07;
/** How far the shadows are lifted before the colors are found. */
const LIFT = 60;
/** The color of skin in the photo, found by its hue and saturation while the colors are still the photo's own. */
const SKIN = [190, 110, 60] as const;
/** How much of a background's dots are ink at full `backgroundDots`, as a share of its area. */
const BACKGROUND_INK = 0.35;
/** Pixels of a part's mask above this value count when finding the part's own color, which leaves out its soft edge. */
const PART_COLOR_THRESHOLD = 200;

/**
 * The colors a print can be made of, all from the comic palette (`Palette.comic()`): two tones of each of its ten inks, and a black, a gray
 * and a white. Every setting that chooses a color holds a place in this list. `place` is where the color is in the palette: the palette
 * has each ink in five tones from dark to light, so the inks are at 2, 7, 13, 19, 23, 27, 32, 38, 41 and 48 and the tone beside each is the
 * one named after it, and its fourteen grays start at 50.
 */
export const POP_ART_COLORS = [
  { label: 'Red', place: 2 },
  { label: 'Coral', place: 3 },
  { label: 'Magenta', place: 7 },
  { label: 'Pink', place: 8 },
  { label: 'Orange', place: 13 },
  { label: 'Burnt orange', place: 12 },
  { label: 'Yellow', place: 19 },
  { label: 'Gold', place: 18 },
  { label: 'Lime', place: 23 },
  { label: 'Olive', place: 22 },
  { label: 'Green', place: 27 },
  { label: 'Mint', place: 28 },
  { label: 'Teal', place: 32 },
  { label: 'Aqua', place: 33 },
  { label: 'Cyan', place: 38 },
  { label: 'Ocean', place: 37 },
  { label: 'Blue', place: 41 },
  { label: 'Periwinkle', place: 42 },
  { label: 'Peach', place: 48 },
  { label: 'Tan', place: 47 },
  { label: 'Black', place: 50 },
  { label: 'Gray', place: 56 },
  { label: 'White', place: 63 },
];

/** The place in `POP_ART_COLORS` of the color with this name. */
const choiceOf = (label: string) => POP_ART_COLORS.findIndex(each => each.label === label);

/** The comic palette and its colors, read once: the palette is the library's, so the app does not carry a copy of its colors. */
let comic: { palette: PaletteLike; colors: ColorLike[] } | undefined;
const comicPalette = () => {
  if (!comic) {
    const palette = Palette.comic();
    comic = { palette, colors: palette.colors() };
  }
  return comic;
};

/** The setting of a part that has not been given a color: it starts as the nearest print color to the color the part has in the photo. */
export const FROM_PHOTO = -1;

/** The setting of dots that have not been given a color: they start as a vivid relative of the color they are on (see `vividRelativeTo`). */
export const VIVID_RELATIVE = -1;

/** The color at a place in `POP_ART_COLORS`. It is kept for good, so it is not freed with what an effect makes. */
export const popArtColor = (choice: number): ColorLike =>
  comicPalette().colors[POP_ART_COLORS[Math.max(0, Math.min(POP_ART_COLORS.length - 1, Math.round(choice)))].place];

/** What `vividRelativeTo` found for each color, kept for good. */
const vividRelative = new Map<number, ColorLike | undefined>();
/** A color with less chroma (CIE Lab) than this has no hue to speak of, so it has no family of its own. */
const LEAST_CHROMA_WITH_A_HUE = 8;
/** How far apart, in degrees of hue, two colors of one family may be. */
const HUE_FAMILY = 40;
/** How much more chroma a color must have than another to be called more vivid. */
const MORE_VIVID_BY = 1;

/**
 * A color of `POP_ART_COLORS` to lay on the color at `choice`, as dots: its closest relative that is more vivid. That is the closest, by CIE Lab
 * distance, of the colors of its hue family (within `HUE_FAMILY` degrees) that have more chroma, so dots on peach are coral and dots on
 * orange are red, never a brown that is only a darker peach or a vivid color of another hue. A color with no hue (black, gray, white), or none
 * that is more vivid in its family, has no such relative, and gets the closest color of any kind instead.
 */
const vividRelativeTo = (choice: number): ColorLike | undefined => {
  if (!vividRelative.has(choice)) {
    const [lightness, a, b] = popArtColor(choice).lab();
    const [chroma, hue] = [Math.hypot(a, b), (Math.atan2(b, a) * 180) / Math.PI];
    const others = POP_ART_COLORS.flatMap((_, other) => {
      if (other === choice) return [];
      const [otherLightness, otherA, otherB] = popArtColor(other).lab();
      const apart = Math.abs((((Math.atan2(otherB, otherA) * 180) / Math.PI - hue) % 360) + 360) % 360;
      return [
        {
          other,
          related:
            chroma > LEAST_CHROMA_WITH_A_HUE &&
            Math.hypot(otherA, otherB) > chroma + MORE_VIVID_BY &&
            Math.min(apart, 360 - apart) <= HUE_FAMILY,
          distance: Math.hypot(otherLightness - lightness, otherA - a, otherB - b),
        },
      ];
    });
    const pool = others.some(each => each.related) ? others.filter(each => each.related) : others;
    const nearest = pool.reduce((best, each) => (each.distance < best.distance ? each : best), pool[0]);
    vividRelative.set(choice, nearest && popArtColor(nearest.other));
  }
  return vividRelative.get(choice);
};

/** The colors of `POP_ART_COLORS` that have no hue. */
const NEUTRAL_COLORS = ['Black', 'Gray', 'White'];
/** A color darker than this (the value of its HSV, from 0 to 1) is taken as black, whatever its hue: dark brown hair is black in the print. */
const DARKEST_WITH_A_HUE = 0.25;
/** A color with less color than this (the saturation of its HSV, from 0 to 1) is taken as a gray, a black or a white. */
const LEAST_SATURATION_WITH_A_HUE = 0.12;

/** The palettes of the print's colors that have a hue and of those that have none, to find the one nearest to a color. Made once. */
let offered: { hued: PaletteLike; neutral: PaletteLike } | undefined;

/**
 * The color of `POP_ART_COLORS` that stands for a color of the photo, which the caller frees. The hue comes first: a color with a noticeable
 * tint becomes the nearest color that has a hue, so a blue-gray skirt is a blue and not the gray that is nearest by distance, and a color that
 * is dark or has hardly any color becomes the nearest black, gray or white.
 */
const popArtColorFor = (color: ColorLike): ColorLike | undefined => {
  if (!offered) {
    const inGroup = (neutral: boolean) =>
      new Palette(
        POP_ART_COLORS.flatMap(({ label }, choice) =>
          NEUTRAL_COLORS.includes(label) === neutral ? [popArtColor(choice)] : [],
        ),
      );
    offered = { hued: inGroup(false), neutral: inGroup(true) };
  }
  const [, saturation, value] = color.hsv();
  const neutral = value < DARKEST_WITH_A_HUE || saturation < LEAST_SATURATION_WITH_A_HUE;
  return (neutral ? offered.neutral : offered.hued).nearest(color);
};

/** The named colors as the controls show them: each name and its color as `#rrggbb`. Made once. */
let swatches: { label: string; css: string }[] | undefined;
export const popArtSwatches = () =>
  (swatches ??= POP_ART_COLORS.map(({ label }, choice) => ({
    label,
    css: `#${[...new Uint8Array(popArtColor(choice).rgb())].map(channel => channel.toString(16).padStart(2, '0')).join('')}`,
  })));

/** Everything that can be changed in a pop art print. Each is from 0 to 1 unless it says otherwise. */
export type PopArtOptions = {
  /** How much of the print shows: 0 is the photo and 1 is the print. */
  strength: number;
  /** The most flat colors a photo is boiled down to (2 to 12), and how alike two may be and still become one. Used when the people are not cut out by part. */
  colors: number;
  merge: number;
  /** The color of the background of a cutout, chosen from `POP_ART_COLORS`. Its dots: how big they get (0 for none), how far apart they are and how dark. */
  background: number;
  backgroundDots: number;
  backgroundDotSpacing: number;
  backgroundDotColor: number;
  /** The flat color of the skin, chosen from `POP_ART_COLORS`, and of the hair, the clothes and what is worn, which are `FROM_PHOTO` until one is chosen. */
  skin: number;
  hair: number;
  clothes: number;
  accessories: number;
  /** The dots on the skin: how big they get (0 for none), how far apart they are, and their color, which is a `VIVID_RELATIVE` of the skin's until one is chosen. */
  dots: number;
  dotSpacing: number;
  dotColor: number;
  /** The lines: how dark, how thick, how much each shape is simplified, and how sharp a turn is kept as a corner. */
  outlines: number;
  lineWidth: number;
  smoothness: number;
  corners: number;
  lineColor: number;
  /** The darkest part of the people that is turned black (a share, up to 0.6), and how black. */
  shadows: number;
  shadowsStrength: number;
  shadowsColor: number;
};

/** What each setting starts as: a bold poster, with a yellow background and a person in peach, black, blue and red. */
export const POP_ART_DEFAULTS: PopArtOptions = {
  strength: 1,
  colors: 8,
  merge: 0.5,
  background: choiceOf('Yellow'),
  backgroundDots: 0,
  backgroundDotSpacing: 0.3,
  backgroundDotColor: VIVID_RELATIVE,
  skin: choiceOf('Peach'),
  hair: FROM_PHOTO,
  clothes: FROM_PHOTO,
  accessories: FROM_PHOTO,
  dots: 1,
  dotSpacing: 0.1,
  dotColor: VIVID_RELATIVE,
  outlines: 1,
  lineWidth: 1,
  smoothness: 0.5,
  corners: 0.5,
  lineColor: choiceOf('Black'),
  shadows: 0.3,
  shadowsStrength: 1,
  shadowsColor: choiceOf('Black'),
};

/**
 * Turns `photo` into a pop art print in place, like a screen print, as `options` say. At 0 `strength` the photo is unchanged and at 1 it is
 * fully pop art. Values in between fade from one to the other.
 *
 * With a `cutout` from `subjectMask` that has the parts of the people it is made by parts, as Photoshop's guides make it: the background,
 * the skin, the hair, the clothes and what is worn are each filled with a flat color, the skin gets halftone dots, the darkest part of
 * the people is turned black, and lines are traced along the edge of every part. With only the people's cutout the background is one flat
 * color and the rest is made as without one. Without a cutout the photo is smoothed, its colors are merged into at most `colors` flat ones
 * and each is replaced by the nearest color of the comic palette; the skin is found by its color and filled and given dots, and a line is
 * drawn where one flat color meets the next.
 */
export const popArt = (photo: AbraImageLike, options: PopArtOptions, cutout?: Record<string, MaskLike>): void =>
  owning(keep => {
    if (options.strength <= 0) return;
    const short = Math.min(photo.width(), photo.height());
    // How thick the lines are, in pixels.
    const line = short * (THINNEST_LINE + options.lineWidth * (THICKEST_LINE - THINNEST_LINE));
    const subject = cutout?.subject;
    // A color of the comic palette is shared, so it is not kept to be freed here.
    const solid = (choice: number) => keep(Fill.solid(popArtColor(choice)));
    // The flat color of a part of the people (the hair, the clothes, what is worn): the color chosen for it, or until one is chosen the
    // print color that stands for the color the part has in the photo (hue first, see `popArtColorFor`), so a person's hair starts as their
    // hair color and no color outside the palette is ever used. Nothing when there is no choice and the mask finds nothing of the part, which is then not painted.
    const partFill = (mask: MaskLike, choice: number) => {
      if (choice >= 0) return solid(choice);
      const own = mask.colorIn(photo, ColorStat.Median, PART_COLOR_THRESHOLD);
      if (!own) return undefined;
      keep(own);
      const nearest = popArtColorFor(own);
      return nearest ? keep(Fill.solid(keep(nearest))) : undefined;
    };
    // The ink of the dots on a color: the color chosen for them, or until one is chosen its vivid relative in the palette.
    const dotInk = (chosen: number, on: number) =>
      chosen >= 0 ? popArtColor(chosen) : (vividRelativeTo(on) ?? popArtColor(on));
    // A color of the palette at some strength, for the lines and the shadows: its alpha is how strongly it is drawn over the print.
    const ink = (choice: number, alpha: number) => {
      const [red, green, blue] = new Uint8Array(popArtColor(choice).rgb());
      return keep(Fill.solid(keep(Color.fromRgba(red, green, blue, Math.round(alpha * 255)))));
    };
    // How big the people are, which the dots and the lines are sized by so that a person who is small in the photo is not drawn with big dots.
    const box = subject?.bounds(128);
    const size = box ? Math.sqrt(box.width * box.height) : short;
    // How far apart dots are, in pixels, for a spacing setting.
    const cellFor = (spacing: number) =>
      Math.max(4, Math.round(size * (CLOSEST_DOTS + spacing * (WIDEST_DOTS - CLOSEST_DOTS))));
    // Dots get bigger where the tone is darker, so a lighter tone gives smaller dots: `dots` is how much of the tone the dots see.
    const dottedTone = (tone: AbraImageLike) => {
      if (options.dots >= 1) return tone;
      const lighter = keep(tone.clone());
      const white = keep(Image.newFromColor(tone.width(), tone.height(), keep(Color.white())));
      keep(keep(blend(white)).withOpacity(1 - options.dots)).apply(lighter);
      return lighter;
    };
    // A halftone of `tone` in this ink on this paper, as a picture to fill dots from.
    const dotsOf = (tone: AbraImageLike, inkColor: ColorLike, paperColor: ColorLike, spacing: number) =>
      keep(keep(keep(keep(halftone(cellFor(spacing))).withInk(inkColor)).withPaper(paperColor)).applied(tone));
    // Dots all the same size over the background: a halftone of a flat gray, a share of which is ink.
    const backgroundDots = (backdropMask: MaskLike, print: AbraImageLike) => {
      if (options.backgroundDots <= 0) return;
      const gray = Math.round(255 * (1 - BACKGROUND_INK * options.backgroundDots));
      const flat = keep(Image.newFromColor(print.width(), print.height(), keep(Color.fromRgb(gray, gray, gray))));
      backdropMask.fillImage(
        print,
        keep(
          Fill.image(
            dotsOf(
              flat,
              dotInk(options.backgroundDotColor, options.background),
              popArtColor(options.background),
              options.backgroundDotSpacing,
            ),
          ),
        ),
      );
    };

    if (cutout && subject && cutout.hair && cutout.skin && cutout.clothes && cutout.accessories) {
      const print = keep(photo.clone());
      keep(Mask.newFromImage(print)).fillImage(print, solid(options.background));
      // Clothes and what is worn go first and the skin last, since the masks of neighbors can overlap a little where they meet.
      for (const [part, choice] of [
        [cutout.clothes, options.clothes],
        [cutout.accessories, options.accessories],
        [cutout.hair, options.hair],
      ] as const) {
        const fill = partFill(part, choice);
        if (fill) part.fillImage(print, fill);
      }
      cutout.skin.fillImage(print, solid(options.skin));

      // The tone of the photo, evened out: where it is dark enough, the print is black.
      const tone = keep(photo.clone());
      keep(median(Math.max(1, short * 0.004))).apply(tone);
      keep(grayscale()).apply(tone);
      keep(autoTone()).apply(tone);
      const level = subject.brightnessPercentile(tone, options.shadows, 127);

      backgroundDots(keep(subject.inverted()), print);
      if (options.dots > 0) {
        cutout.skin.fillImage(
          print,
          keep(
            Fill.image(
              dotsOf(
                dottedTone(tone),
                dotInk(options.dotColor, options.skin),
                popArtColor(options.skin),
                options.dotSpacing,
              ),
            ),
          ),
        );
      }
      const detail = keep(keep(Mask.fromBrightnessRange(tone, 0, level, 0)).multiplied(subject));
      if (options.shadows > 0) detail.fillImage(print, ink(options.shadowsColor, options.shadowsStrength));

      // The lines are traced along the edge of each part and of the black detail, smoothed, and drawn: one call for each, with no points
      // copied out, at the size of the picture, so they stay sharp. `outlines` is how dark they are.
      if (options.outlines > 0) {
        const tolerance = Math.max(1, size * options.smoothness * MOST_TRACE_TOLERANCE);
        const thickness = Math.max(1.5, line);
        const corners = options.corners * MOST_CORNER_DEGREES;
        let lines: MaskLike | undefined;
        for (const part of [cutout.hair, cutout.skin, cutout.clothes, cutout.accessories, detail]) {
          const drawn = keep(part.outlined(128, tolerance, corners, thickness));
          lines = lines ? keep(lines.maximum(drawn)) : drawn;
        }
        lines?.fillImage(print, ink(options.lineColor, options.outlines));
      }
      keep(keep(blend(print)).withOpacity(options.strength)).apply(photo);
      return;
    }

    const print = keep(photo.clone());
    highlightsShadows(print, 0, LIFT);
    keep(surfaceBlur(Math.max(2, Math.round(short * SMOOTHING)), SMOOTH_LIMIT)).apply(print);
    keep(median(Math.max(1, short * CLEAN))).apply(print);
    const smoothed = keep(print.clone());

    // The tone before the colors are merged: the merge makes the skin one color, but the dots are bigger where it is darker.
    const tone = keep(smoothed.clone());
    keep(grayscale()).apply(tone);
    keep(autoTone()).apply(tone);
    keep(contrast(30)).apply(tone);

    const merging = keep(
      keep(keep(quantize(options.colors)).withTolerance(options.merge * MAX_TOLERANCE)).withLightness(LIGHTNESS_WEIGHT),
    );
    const found = merging.palette(print);
    merging.apply(print);

    // Skin is picked out by its color while the colors are still the photo's own, which leaves out the grays of hair and walls.
    const lips = keep(Mask.fromColorRange(smoothed, keep(Color.fromRgb(200, 80, 100)), 16, 0.3, 0.2, 0.2));
    const anySkin = keep(
      keep(keep(Mask.fromColorRange(print, keep(Color.fromRgb(...SKIN)), 28, 0.2, 0.15, 0.1)).opened(1)).subtracted(
        lips,
      ),
    );
    const skin = subject ? keep(anySkin.multiplied(subject)) : anySkin;

    // Each flat color becomes the palette's nearest color, in the same order they were found.
    const palette = keep(Palette.comic());
    const targets = found.map(flat => palette.nearest(flat) ?? flat);
    keep(keep(keep(quantize(0)).withSources(found)).withPalette(targets)).apply(print);
    // The background of a cutout is the chosen flat color, filled before the lines so the edge of the people gets a line too.
    if (subject) {
      const inverted = keep(subject.inverted());
      inverted.fillImage(print, solid(options.background));
      backgroundDots(inverted, print);
    }
    // The skin is the chosen flat color, which the dots are on.
    skin.fillImage(print, solid(options.skin));

    // The lines come from the flat colors, before the dots, so the dots do not make edges of their own.
    const edges =
      options.outlines > 0
        ? keep(keep(keep(colorEdges(Math.max(2, Math.round(line)))).withTolerance(1)).applied(print))
        : null;

    if (options.dots > 0) {
      skin.fillImage(
        print,
        keep(
          Fill.image(
            dotsOf(
              dottedTone(tone),
              dotInk(options.dotColor, options.skin),
              popArtColor(options.skin),
              options.dotSpacing,
            ),
          ),
        ),
      );
    }

    if (edges) keep(keep(blend(edges)).withMode(BlendMode.Multiply).withOpacity(options.outlines)).apply(print);
    keep(keep(blend(print)).withOpacity(options.strength)).apply(photo);
  });
