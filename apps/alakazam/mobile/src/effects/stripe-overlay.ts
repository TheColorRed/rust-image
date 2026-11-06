import { BlendMode } from '@abra/core/blend';
import { Color } from '@abra/core/color';
import { Image, type ImageLike } from '@abra/core/image';
import { Blend, sprites, type SpritesLike } from '@abra/core/sprites';
import { installedFamilies, renderText } from '@/src/lib/fonts/text-image';
import { owning } from '@/src/lib/native/native-handles';

/**
 * Where each setting is in the values the Strength slider saves: its own value, then what the pickers save after it (the picked look, the three
 * switches, where 1 is on) and the words (see `encodeWords`), which are kept after everything else, as the gradient keeps its stops.
 */
export const STRIPE_AT = { strength: 0, look: 1, showSub: 2, capsTitle: 3, capsSub: 4, words: 5 } as const;

/** What the Strength slider and the pickers start at: fully solid, the first colors, and the sub text and the capitals on. */
export const STRIPE_DEFAULTS = { strength: 1, look: 0, showSub: 1, capsTitle: 1, capsSub: 1 } as const;

/**
 * The settings that have a slider of their own, as `stripeOverlay` takes them. The angle is in degrees (a little uphill, as the stripes of a
 * title card are); the rest are shares from 0 to 1: the size of the letters, where the main stripe is down the photo, and where the sub stripe
 * is, down from tucked under the main stripe (`gap`, from -1 above it to 1 below it) and across from the left edge (`shift`), each of the room it has on the photo. The `padding` is how much room the stripes leave above and below their letters, from 0 (a little) to 1 (a lot).
 */
export type StripePlace = { size: number; position: number; angle: number; gap: number; shift: number; padding: number };
export const STRIPE_PLACE_DEFAULTS: StripePlace = { size: 0.5, position: 0.5, angle: -8, gap: 0, shift: 0.75, padding: 0.5 };

/** The words a stripe shows until they are changed. */
export const STRIPE_WORDS = { title: 'Your title', subtitle: 'Subtitle' } as const;

/** One ready-made set of colors: the long stripe, the short stripe under it, and the letters on each. */
export type StripeLook = { label: string; stripe: Rgb; under: Rgb; ink: Rgb; underInk: Rgb };
type Rgb = readonly [red: number, green: number, blue: number];

/** The ready-made colors people choose from, by the number their picker saves. */
export const STRIPE_LOOKS: readonly StripeLook[] = [
  { label: 'Royal', stripe: [26, 86, 188], under: [4, 20, 56], ink: [242, 242, 245], underInk: [255, 255, 255] },
  { label: 'Cherry', stripe: [196, 28, 36], under: [30, 12, 14], ink: [255, 255, 255], underInk: [255, 255, 255] },
  { label: 'Sunny', stripe: [244, 190, 30], under: [60, 40, 10], ink: [40, 28, 6], underInk: [255, 240, 200] },
  { label: 'Orange', stripe: [230, 110, 20], under: [70, 28, 6], ink: [255, 255, 255], underInk: [255, 255, 255] },
  { label: 'Teal', stripe: [24, 170, 180], under: [8, 48, 60], ink: [255, 255, 255], underInk: [255, 255, 255] },
  { label: 'Forest', stripe: [40, 120, 70], under: [10, 36, 22], ink: [255, 255, 255], underInk: [255, 255, 255] },
  { label: 'Plum', stripe: [120, 50, 160], under: [30, 12, 44], ink: [255, 255, 255], underInk: [255, 255, 255] },
  { label: 'Mono', stripe: [245, 245, 245], under: [20, 20, 20], ink: [20, 20, 20], underInk: [245, 245, 245] },
];

/** Every setting of the stripes from the control's saved values, with what is missing filled in. The result is in the places `STRIPE_AT` names. */
export const stripeValues = (values: readonly (number | undefined)[]): number[] => [
  values[STRIPE_AT.strength] ?? STRIPE_DEFAULTS.strength,
  values[STRIPE_AT.look] ?? STRIPE_DEFAULTS.look,
  values[STRIPE_AT.showSub] ?? STRIPE_DEFAULTS.showSub,
  values[STRIPE_AT.capsTitle] ?? STRIPE_DEFAULTS.capsTitle,
  values[STRIPE_AT.capsSub] ?? STRIPE_DEFAULTS.capsSub,
  ...values.slice(STRIPE_AT.words).map(value => value ?? 0),
];

/** What the person typed, and the font it is in (`''` for the first font on the device). */
export type StripeWords = { title: string; subtitle: string; family: string };

/** The words as the numbers saved after the sliders: for the title, the subtitle and the font, how many letters there are and then each letter's code. */
export const encodeWords = ({ title, subtitle, family }: StripeWords): number[] =>
  [title, subtitle, family].flatMap(text => [text.length, ...Array.from({ length: text.length }, (_, at) => text.charCodeAt(at))]);

/** The words from the saved values: what is saved, or the ready-made words when nothing was ever typed. */
export const decodeWords = (values: readonly (number | undefined)[]): StripeWords => {
  const tail = values.slice(STRIPE_AT.words);
  const ready = { ...STRIPE_WORDS, family: '' };
  let at = 0;
  const read: string[] = [];
  for (let each = 0; each < 3; each++) {
    const length = tail[at++];
    // A length that is not a count, or that runs past the end, means what is saved is not text in this layout (a project saved by an older
    // version of the control kept its values in other places), so the ready-made text shows and not numbers read as letters.
    if (length === undefined || !Number.isInteger(length) || length < 0 || at + length > tail.length) return ready;
    read.push(String.fromCharCode(...tail.slice(at, at + length).map(code => code ?? 0)));
    at += length;
  }
  return at === tail.length ? { title: read[0], subtitle: read[1], family: read[2] } : ready;
};

/** The saved values with other words, which replace any typed before. */
export const withWords = (values: readonly (number | undefined)[], words: StripeWords): number[] => [
  ...stripeValues(values).slice(0, STRIPE_AT.words),
  ...encodeWords(words),
];

/**
 * How many times as tall as its letters the stripes are, at the least and the most padding, which is the room above and below the letters a title
 * card leaves. The sub stripe is a little less tall than the main one. At 0 up/down it is tucked `STRIPE_TUCK` of the main stripe's thickness
 * under it, as the dark stripe is on the blue one.
 */
const STRIPE_PADDING = { least: 1.2, most: 3, sub: 0.86 } as const;
const STRIPE_TUCK = 0.2;

/** The shorter side of a stripe's picture, in pixels. A stripe is one color, so its picture is made small and the sprite scales it up; this many pixels keep its shape true. */
const STRIPE_PICTURE = 16;

/** A piece of text on the photo: where its middle is (in pixels of the photo), how big its letters are, and how much it is turned, in degrees. */
export type StripeTextBox = { x: number; y: number; width: number; height: number; angle: number };

/**
 * Works out where everything goes for a photo `width` by `height`: the text pictures and where the stripes and the text are. It makes the text
 * pictures (the caller owns them through `keep`) because how big the letters come out sets how thick the stripes are, which sets where the sub
 * stripe goes. `stripeOverlay` draws from it and `stripeTextBoxes` answers where a tap lands, so the two always agree.
 */
const layoutStripes = (keep: <T>(value: T) => T, width: number, photoHeight: number, values: number[], placed: Partial<StripePlace>) => {
  const [strength, lookAt, showSub, capsTitle, capsSub] = stripeValues(values);
  const { size, position, angle, gap, shift: sideways, padding } = { ...STRIPE_PLACE_DEFAULTS, ...placed };
  const tall = STRIPE_PADDING.least + padding * (STRIPE_PADDING.most - STRIPE_PADDING.least);
  const look = STRIPE_LOOKS[lookAt] ?? STRIPE_LOOKS[0];
  const words = decodeWords(values);
  const family = installedFamilies().some(each => each.family === words.family) ? words.family : (installedFamilies()[0]?.family ?? '');
  const letters = Math.min(width, photoHeight) * (0.05 + 0.1 * size);
  const ink = (rgb: Rgb) => keep(Color.fromRgba(rgb[0], rgb[1], rgb[2], 255));
  const title = family ? renderText({ text: capsTitle ? words.title.toUpperCase() : words.title, family, size: letters, color: ink(look.ink) }) : null;
  // The subtitle is optional: with it off, or with nothing typed, there is no short stripe either.
  const subtitle =
    family && showSub
      ? renderText({ text: capsSub ? words.subtitle.toUpperCase() : words.subtitle, family, size: letters * 0.55, color: ink(look.underInk) })
      : null;
  if (title) keep(title);
  if (subtitle) keep(subtitle);

  const radians = (angle * Math.PI) / 180;
  const [sin, cos] = [Math.sin(radians), Math.cos(radians)];
  const centre = [width / 2, photoHeight * position] as const;
  // The long stripe reaches past the corners at any angle. Without a title it is as thick as a line of letters would make it.
  const longStripe = { length: Math.hypot(width, photoHeight) * 1.3, thick: (title?.height() ?? letters) * tall };
  const shortThick = (subtitle?.height() ?? letters * 0.55) * tall * STRIPE_PADDING.sub;
  const shortLength = subtitle ? subtitle.width() + shortThick : 0;

  // The short stripe hangs under the long one and is placed by where its whole turned shape can go on the photo: across from 0 (its text's left end at
  // the photo's left edge) to 1 (its text's right end at the right edge, with the stripe running on off the photo), and down from 0 (tucked under the long stripe's edge) to 1 (its lower edge on the
  // photo's lower edge), or up to -1 (its upper edge on the photo's upper edge, above the long stripe). The text's size is counted, so it never leaves the photo however long it is.
  let under: { x: number; y: number } | null = null;
  if (subtitle) {
    const halfAcross = (shortLength * Math.abs(cos) + shortThick * Math.abs(sin)) / 2;
    const halfDown = (shortLength * Math.abs(sin) + shortThick * Math.abs(cos)) / 2;
    const between = (low: number, high: number, share: number) => (high < low ? (low + high) / 2 : low + share * (high - low));
    const x = between(halfAcross, width - halfAcross, sideways);
    // How far the short stripe's middle is from the long one's when it overlaps the long stripe's edge by `STRIPE_TUCK`, across the stripes.
    const tucked = (longStripe.thick + shortThick) / 2 - longStripe.thick * STRIPE_TUCK;
    // Where its middle is then, straight down from the long stripe's middle and then along it to `x`.
    const touching = centre[1] + Math.tan(radians) * (x - centre[0]) + tucked / cos;
    // Above the long stripe it is the same in the other direction: tucked under its upper edge, then up to the photo's upper edge.
    const touchingAbove = touching - (2 * tucked) / cos;
    const y = gap >= 0 ? touching + gap * (Math.max(touching, photoHeight - halfDown) - touching) : touchingAbove + -gap * (Math.min(touchingAbove, halfDown) - touchingAbove);
    under = { x, y };
  }
  return { strength, look, angle, sin, cos, ink, title, subtitle, centre, longStripe, shortThick, shortLength, under };
};

/**
 * Where the main text and the sub text are on a photo `width` by `height`, for the saved `values` and the settings with sliders of their own
 * (`placed`), so a tap on the photo can be matched to the text under it. The text is left out when it is not shown (no font, or the sub text is off).
 */
export const stripeTextBoxes = (width: number, height: number, values: number[], placed: Partial<StripePlace> = {}): { main?: StripeTextBox; sub?: StripeTextBox } =>
  owning(keep => {
    const { angle, title, subtitle, centre, under } = layoutStripes(keep, width, height, values, placed);
    return {
      main: title ? { x: centre[0], y: centre[1], width: title.width(), height: title.height(), angle } : undefined,
      sub: subtitle && under ? { x: under.x, y: under.y, width: subtitle.width(), height: subtitle.height(), angle } : undefined,
    };
  });

/**
 * Lays the stripes over `photo` in place, from the control's saved values (see `STRIPE_AT`): a long stripe across the photo with the title on
 * it, and a shorter stripe under it with the subtitle. The photo remembers the effect and the GPU draws it, so this does no image work on the
 * JavaScript thread. `placed` is the settings that have sliders of their own. Sizes and places are shares of the photo, so the same values give the same stripes on a small copy and the full photo.
 */
export const stripeOverlay = (photo: ImageLike, values: number[], placed: Partial<StripePlace> = {}): void =>
  owning(keep => {
    const width = photo.width();
    const photoHeight = photo.height();
    const { strength, look, angle, sin, cos, ink, title, subtitle, centre, longStripe, shortThick, shortLength, under } = layoutStripes(keep, width, photoHeight, values, placed);
    if (strength <= 0) return;

    // One sprite as the numbers the library takes: where, half its longer side, how solid, the tint (white leaves the picture as it is), how much
    // it is turned, and the values that are not used.
    const sprite = (x: number, y: number, longer: number) => [x, y, longer / 2, strength, 255, 255, 255, 1, angle, 0, 0, 0, 0, 1, 1, 0];
    const normal = keep(Blend.mode(BlendMode.Normal));
    let drawn: SpritesLike = keep(sprites());
    const add = (picture: ImageLike, x: number, y: number) => {
      drawn = keep(drawn.addSpriteValues(sprite(x, y, Math.max(picture.width(), picture.height())), normal, picture));
    };
    // A stripe picture has the stripe's shape at a small size, and the sprite scales it up; its size is the sprite's half longer side.
    const stripe = (rgb: Rgb, length: number, thick: number, x: number, y: number) => {
      const scale = STRIPE_PICTURE / Math.min(length, thick);
      const picture = keep(Image.newFromColor(Math.max(1, Math.round(length * scale)), Math.max(1, Math.round(thick * scale)), ink(rgb)));
      drawn = keep(drawn.addSpriteValues(sprite(x, y, Math.max(length, thick)), normal, picture));
    };
    stripe(look.stripe, longStripe.length, longStripe.thick, centre[0], centre[1]);
    if (subtitle && under) {
      // The stripe starts where the text's place starts and runs off the right edge of the photo, as the long stripe runs off both, so wherever the
      // text is the stripe is never seen to end on that side.
      const start = [under.x - (cos * shortLength) / 2, under.y - (sin * shortLength) / 2] as const;
      const reach = Math.max(shortLength, (width - start[0]) / cos + shortThick);
      stripe(look.under, reach, shortThick, start[0] + (cos * reach) / 2, start[1] + (sin * reach) / 2);
      add(subtitle, under.x, under.y);
    }
    if (title) add(title, centre[0], centre[1]);
    drawn.applyDeferred(photo);
  });
