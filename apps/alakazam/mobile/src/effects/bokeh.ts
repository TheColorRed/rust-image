import { owning } from '@/src/lib/native/native-handles';
import { BlendMode, blend } from '@abra/core/blend';
import { Color } from '@abra/core/color';
import { Fill, fillArea, type FillLike } from '@abra/core/fill';
import { gaussianBlur } from '@abra/core/filters';
import { Area, Shape as AbraShape, aspectRatioMeet } from '@abra/core/geometry';
import { Image, type ImageLike } from '@abra/core/image';
import { type AbraImageLike } from '@alakazam/mobile';
import Rand from 'rand-seed';

/** The base for the orb layout. Layout 0 uses it as is, and other layouts add a number to it. */
const BASE_LAYOUT = 'alakazam';
/** The saturation of the orb colours. */
const SATURATION = 0.7;
/** The weakest orb's strength, as a share of `amount`. Every orb is between this share and the full `amount`. */
const MIN_STRENGTH = 0.3;

/**
 * Turns a position on the hue bar, from 0 to 1, into an orb colour at full brightness. The bar runs the wheel backwards (red,
 * then magenta, then blue), so the position is turned around to give the colour the bar shows.
 */
const hueToRgb = (position: number): [number, number, number] => {
  const hue = 1 - position;
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
 * the photo. The softer layers hold bigger orbs, which reads as the farther light. The orbs of a kind take the layers in turn.
 */
const TIERS = [
  { blur: 0.008, minRadius: 0.02, maxRadius: 0.05 },
  { blur: 0.016, minRadius: 0.04, maxRadius: 0.09 },
  { blur: 0.03, minRadius: 0.07, maxRadius: 0.14 },
];

/** Scales an orb color's brightness. In Screen mode black adds no light, so a dimmer orb is fainter. */
const scaleRgb = (rgb: [number, number, number], scale: number): [number, number, number] => [
  Math.round(rgb[0] * scale),
  Math.round(rgb[1] * scale),
  Math.round(rgb[2] * scale),
];

/** The possible shapes for the bokeh orbs. The circle is drawn directly and each other one is a predefined shape of the library. */
export type Shape =
  'circle' | 'triangle' | 'diamond' | 'heart' | 'star' | 'arrow' | 'half-moon' | 'crescent-moon' | 'cross';

/** The shapes `'random'` picks from, one for each orb. */
const SHAPES: Shape[] = [
  'circle',
  'triangle',
  'diamond',
  'heart',
  'star',
  'arrow',
  'half-moon',
  'crescent-moon',
  'cross',
];

/** The rotation that means each orb gets a random angle. */
export const RANDOM_ROTATION = 360;

/** The most orbs of one kind the range slider allows. */
export const MAX_ORBS = 20;

/** Every choice of shape, in the order the control saves them: the control keeps the place of the one chosen. */
export const BOKEH_SHAPES: (Shape | 'random')[] = [...SHAPES, 'random'];

/** The library's shape for each shape other than the circle. */
const LIBRARY_SHAPES: Record<Exclude<Shape, 'circle'>, () => AbraShape> = {
  triangle: () => new AbraShape.Triangle(),
  diamond: () => new AbraShape.Diamond(),
  heart: () => new AbraShape.Heart(),
  star: () => new AbraShape.Star(),
  arrow: () => new AbraShape.Arrow(),
  'half-moon': () => new AbraShape.HalfMoon(),
  'crescent-moon': () => new AbraShape.CrescentMoon(),
  cross: () => new AbraShape.Cross(),
};

/**
 * Draws one orb of `color` on `layer`. A shape is made to fit a box as wide as the orb, then drawn with its middle on `center`,
 * so it takes the place a circle of `radius` would. `rotation` turns the shape clockwise around that middle, in degrees.
 */
const drawOrb = (
  keep: <T>(handle: T) => T,
  layer: ImageLike,
  shape: Shape,
  center: { x: number; y: number },
  radius: number,
  orbFill: FillLike,
  rotation: number,
): void => {
  if (shape === 'circle') {
    keep(fillArea(keep(Area.circle(center, radius)), orbFill)).apply(layer);
    return;
  }
  const fitted = keep(
    keep(Area.shape(LIBRARY_SHAPES[shape]())).fit({ width: radius * 2, height: radius * 2 }, aspectRatioMeet()),
  );
  const bounds = fitted.bounds();
  const radians = (rotation * Math.PI) / 180;
  const cos = Math.cos(radians);
  const sin = Math.sin(radians);
  // The outline is turned by hand, since an area has no rotation of its own, and put with its middle on `center`.
  const turned = fitted.flatten(0.25).map(point => {
    const dx = point.x - (bounds.x + bounds.width / 2);
    const dy = point.y - (bounds.y + bounds.height / 2);
    return [center.x + dx * cos - dy * sin, center.y + dx * sin + dy * cos];
  });
  keep(fillArea(keep(Area.fromPoints(turned)), orbFill)).apply(layer);
};

/** What one kind of orb looks like. */
export type OrbKind = {
  /** The color, as a position from 0 to 1 on the hue bar. */
  hue: number;
  shape: Shape | 'random';
  /** The fewest and the most orbs of this kind: the layout picks a number between them. */
  minOrbs: number;
  maxOrbs: number;
  /** Turns every shape by this many degrees, from 0 to 359; `RANDOM_ROTATION` gives each orb its own angle. */
  rotation: number;
  /** The strength of the strongest orb, from 0 to 1. Each orb is randomly weaker, down to `MIN_STRENGTH` of it. */
  strength: number;
};

/**
 * Adds soft, out of focus light orbs to `photo` in place, screened over the image so dark areas stay dark. The orbs are
 * placed from random sequences that start from the layout, so the same layout always gives the same arrangement. `layout`
 * picks the arrangement: 0 is the base layout and each other value gives a different one. Each kind of orb has its own color,
 * shape, number, rotation and strength. A kind with 0 strength adds nothing.
 */
export const bokeh = (photo: AbraImageLike, layout: number, kinds: OrbKind[]): void =>
  owning(keep => {
    const width = photo.width();
    const height = photo.height();
    const short = Math.min(width, height);
    const seed = layout === 0 ? BASE_LAYOUT : `${BASE_LAYOUT}-${layout}`;
    // Each kind has its own sequences, so one kind's settings never move the orbs of another. The orb at a place in a kind
    // keeps its position, so a larger number only adds orbs.
    const orbs = kinds.flatMap((kind, index) => {
      if (kind.strength <= 0) return [];
      const rand = new Rand(`${seed}-${index}`);
      const strengthRand = new Rand(`${seed}-${index}-strength`);
      const shapeRand = new Rand(`${seed}-${index}-shape`);
      const turnRand = new Rand(`${seed}-${index}-turn`);
      const count = kind.minOrbs + Math.floor(rand.next() * (kind.maxOrbs - kind.minOrbs + 1));
      const base = hueToRgb(kind.hue);
      return Array.from({ length: count }, (_, i) => {
        const tier = i % TIERS.length;
        const { minRadius, maxRadius } = TIERS[tier];
        const radius = short * (minRadius + (maxRadius - minRadius) * rand.next());
        const center = { x: rand.next() * width, y: rand.next() * height };
        const strength = kind.strength * (MIN_STRENGTH + (1 - MIN_STRENGTH) * strengthRand.next());
        const shape = kind.shape === 'random' ? SHAPES[Math.floor(shapeRand.next() * SHAPES.length)] : kind.shape;
        const turn = kind.rotation >= RANDOM_ROTATION ? turnRand.next() * 360 : kind.rotation;
        return { tier, radius, center, shape, turn, rgb: scaleRgb(base, strength) };
      });
    });
    if (orbs.length === 0) return;
    const black = keep(Color.fromRgb(0, 0, 0));
    TIERS.forEach((tier, tierIndex) => {
      const inTier = orbs.filter(orb => orb.tier === tierIndex);
      if (inTier.length === 0) return;
      const layer = keep(Image.newFromColor(width, height, black));
      for (const orb of inTier) {
        const orbFill = keep(Fill.solid(keep(Color.fromRgb(...orb.rgb))));
        drawOrb(keep, layer, orb.shape, orb.center, orb.radius, orbFill, orb.turn);
      }
      keep(gaussianBlur(Math.max(2, short * tier.blur))).apply(layer);
      keep(keep(blend(layer)).withMode(BlendMode.Screen)).apply(photo);
    });
  });
