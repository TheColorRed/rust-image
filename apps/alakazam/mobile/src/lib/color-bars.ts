import { Color } from '@abra/core/color';
import { Gradient } from '@abra/core/gradient';
import { owning } from '@/src/lib/faces';

/**
 * The colors of the bars that pick a color (`picker` of a slider control), each as red, green and blue from 0 to 255. The bar
 * draws these evenly spaced, and a tool that is given a position on the bar reads the color from the same gradient, so the two
 * cannot disagree.
 *
 * What is made by the library is made when it is first asked for, not when this file loads: the library is not ready until the
 * app has started, and the controls import this file before that.
 */

const once = <T>(make: () => T) => {
  let made: T | undefined;
  return () => (made ??= make());
};

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
export const shadeGradient = once(() =>
  Gradient.evenly(SHADE_COLORS.map(([red, green, blue]) => Color.fromRgb(red, green, blue))),
);

/** The tints of a highlighter: pearl, champagne, gold, peach and rose, from the palest to the pinkest. */
export const GLOW_COLORS = [
  [255, 250, 245],
  [247, 231, 206],
  [234, 200, 120],
  [255, 190, 150],
  [244, 170, 180],
];
export const glowGradient = once(() =>
  Gradient.evenly(GLOW_COLORS.map(([red, green, blue]) => Color.fromRgb(red, green, blue))),
);

/** The tan scale, from no tan to the darkest: white, tan, light brown, brown, dark brown and black. */
export const TAN_COLORS = [
  [255, 255, 255],
  [210, 180, 140],
  [181, 101, 29],
  [139, 90, 43],
  [101, 67, 33],
  [0, 0, 0],
];
export const tanGradient = once(() =>
  Gradient.evenly(TAN_COLORS.map(([red, green, blue]) => Color.fromRgb(red, green, blue))),
);
