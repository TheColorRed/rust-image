import { owning } from '@/src/lib/native/native-handles';
import { BlendMode, blend } from '@abra/core/blend';
import { Color } from '@abra/core/color';
import { Fill, fillArea } from '@abra/core/fill';
import { gaussianBlur } from '@abra/core/filters';
import { Area } from '@abra/core/geometry';
import { Image } from '@abra/core/image';
import { type AbraImageLike } from '@alakazam/mobile';
import Rand from 'rand-seed';

/** The base for the orb layout. Layout 0 uses it as is, and other layouts add a number to it. */
const BASE_LAYOUT = 'alakazam';
/** The saturation of the orb colours. */
const SATURATION = 0.7;
/** The weakest orb's strength, as a share of `amount`. Every orb is between this share and the full `amount`. */
const MIN_STRENGTH = 0.3;

/** Turns a hue, from 0 to 1 around the colour wheel, into an orb colour at full brightness. */
const hueToRgb = (hue: number): [number, number, number] => {
  const h = (hue - Math.floor(hue)) * 6;
  const c = SATURATION;
  const x = c * (1 - Math.abs((h % 2) - 1));
  const sectors: [number, number, number][] = [
    [c, x, 0],
    [x, c, 0],
    [0, c, x],
    [0, x, c],
    [x, 0, c],
    [c, 0, x],
  ];
  const [r, g, b] = sectors[Math.floor(h)];
  const m = 1 - c;
  return [Math.round((r + m) * 255), Math.round((g + m) * 255), Math.round((b + m) * 255)];
};

/**
 * The depth layers, from sharp to soft. Sizes and blurs are fractions of the photo's shorter side, so the look scales with
 * the photo. The softer layers hold bigger orbs, which reads as the farther light. Each layer's orb count is picked from
 * `minCount` to `maxCount` by the layout.
 */
const TIERS = [
  { blur: 0.008, minRadius: 0.02, maxRadius: 0.05, minCount: 3, maxCount: 6 },
  { blur: 0.016, minRadius: 0.04, maxRadius: 0.09, minCount: 2, maxCount: 5 },
  { blur: 0.03, minRadius: 0.07, maxRadius: 0.14, minCount: 1, maxCount: 4 },
];

/** Scales an orb colour's brightness. In Screen mode black adds no light, so a dimmer orb is fainter. */
const scaleRgb = (rgb: [number, number, number], scale: number): [number, number, number] =>
  [Math.round(rgb[0] * scale), Math.round(rgb[1] * scale), Math.round(rgb[2] * scale)];

/**
 * Adds soft, out of focus light orbs to `photo` in place, screened over the image so dark areas stay dark. The orbs are
 * placed from a random sequence that starts from the layout, so the same layout always gives the same arrangement. `layout`
 * picks the arrangement, including how many orbs each layer has: 0 is the base layout and each other value gives a different
 * one. The orbs take one of the two colors, given as hues from 0 to 1. `amount` is the strength of the strongest orb. Each
 * orb is randomly weaker, down to `MIN_STRENGTH` of `amount`. At 0 amount the photo is unchanged.
 */
export const bokeh = (photo: AbraImageLike, amount: number, firstHue: number, secondHue: number, layout: number): void =>
  owning(keep => {
    if (amount <= 0) return;
    const width = photo.width();
    const height = photo.height();
    const short = Math.min(width, height);
    const seed = layout === 0 ? BASE_LAYOUT : `${BASE_LAYOUT}-${layout}`;
    const rand = new Rand(seed);
    const strengthRand = new Rand(`${seed}-strength`);
    const colors = [hueToRgb(firstHue), hueToRgb(secondHue)];
    const black = keep(Color.fromRgb(0, 0, 0));
    for (const tier of TIERS) {
      const layer = keep(Image.newFromColor(width, height, black));
      const count = tier.minCount + Math.floor(rand.next() * (tier.maxCount - tier.minCount + 1));
      for (let i = 0; i < count; i++) {
        const radius = short * (tier.minRadius + (tier.maxRadius - tier.minRadius) * rand.next());
        const center = { x: rand.next() * width, y: rand.next() * height };
        const base = colors[rand.next() < 0.5 ? 0 : 1];
        const strength = amount * (MIN_STRENGTH + (1 - MIN_STRENGTH) * strengthRand.next());
        const color = keep(Color.fromRgb(...scaleRgb(base, strength)));
        const orbFill = keep(Fill.solid(color));
        const orb = keep(Area.circle(center, radius));
        keep(fillArea(orb, orbFill)).apply(layer);
      }
      keep(gaussianBlur(Math.max(2, short * tier.blur))).apply(layer);
      keep(keep(blend(layer)).withMode(BlendMode.Screen)).apply(photo);
    }
  });
