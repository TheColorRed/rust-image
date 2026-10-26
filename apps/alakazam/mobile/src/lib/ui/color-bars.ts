import { Color } from '@abra/core/color';
import { Gradient } from '@abra/core/gradient';
import { keptForLife } from '@/src/lib/native/handle-ledger';
import { owning } from '@/src/lib/native/native-handles';

/**
 * The colors of the bars that pick a color (`picker` of a slider control), each as red, green and blue from 0 to 255. The bar
 * draws these evenly spaced, and a tool that is given a position on the bar reads the color from the same gradient, so the two
 * cannot disagree.
 *
 * What is made by the library is made when it is first asked for, not when this file loads: the library is not ready until the
 * app has started, and the controls import this file before that. It is then kept for as long as the app runs.
 */

const once = <T>(make: () => T) => {
  let made: T | undefined;
  return () => (made ??= keptForLife(make()));
};

/** A gradient with these colors evenly spaced. The gradient copies the colors, so the handles made for them are freed here. */
const evenly = (colors: number[][]) =>
  owning(keep => Gradient.evenly(colors.map(([red, green, blue]) => keep(Color.fromRgb(red, green, blue)))));

/** The hues of the color wheel, 360 at 0 falling to 0 at 1: a stop every 30 degrees is as exact as the wheel itself. */
export const hueColors = once(() =>
  owning(keep => {
    const wheel = keep(Gradient.hue());
    return Array.from({ length: 13 }, (_, step) => Array.from(new Uint8Array(keep(wheel.colorAt(step / 12)).rgb())));
  }),
);

/** The shades of a shadow: a light tan, a light brown, a brown and a dark brown (the library's `Color::tan`, `light_brown`, `brown` and `dark_brown`). */
export const SHADE_COLORS = [
  [210, 180, 140],
  [181, 101, 29],
  [139, 90, 43],
  [101, 67, 33],
];
export const shadeGradient = once(() => evenly(SHADE_COLORS));

/** The tints of a highlighter: pearl, champagne, gold, peach and rose, from the palest to the pinkest. */
export const GLOW_COLORS = [
  [255, 250, 245],
  [247, 231, 206],
  [234, 200, 120],
  [255, 190, 150],
  [244, 170, 180],
];
export const glowGradient = once(() => evenly(GLOW_COLORS));

/** The infrared tints, from hot pink foliage to snowy white. */
export const INFRARED_COLORS = [
  [230, 30, 140],
  [255, 120, 190],
  [255, 210, 230],
  [255, 255, 255],
];
export const infraredGradient = once(() => evenly(INFRARED_COLORS));

/** The neon tints of the vaporwave grid, from teal through blue and violet to hot pink. */
export const NEON_COLORS = [
  [90, 240, 255],
  [60, 140, 255],
  [150, 80, 255],
  [255, 60, 200],
];

/** The tan scale, from no tan to the darkest: white, tan, light brown, brown, dark brown and black. */
export const TAN_COLORS = [
  [255, 255, 255],
  [210, 180, 140],
  [181, 101, 29],
  [139, 90, 43],
  [101, 67, 33],
  [0, 0, 0],
];
export const tanGradient = once(() => evenly(TAN_COLORS));

/** The color at `position` (0 to 1) along a bar of evenly spaced `stops`, as red, green and blue, mixed between the two stops either side. */
export const colorAlongBar = (stops: number[][], position: number): number[] => {
  const at = Math.min(Math.max(position, 0), 1) * (stops.length - 1);
  const low = Math.floor(at);
  const high = Math.min(low + 1, stops.length - 1);
  const mix = at - low;
  return stops[low].map((channel, index) => Math.round(channel + (stops[high][index] - channel) * mix));
};

