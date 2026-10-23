import { Color } from '@abra/core/color';
import { Gradient } from '@abra/core/gradient';

const shadeStops = [
  [210, 180, 140],
  [181, 101, 29],
  [139, 90, 43],
  [101, 67, 33],
];
const glowStops = [
  [255, 250, 245],
  [247, 231, 206],
  [234, 200, 120],
  [255, 190, 150],
  [244, 170, 180],
];
const shade = Gradient.evenly(shadeStops.map(([red, green, blue]) => Color.fromRgb(red, green, blue)));
const glow = Gradient.evenly(glowStops.map(([red, green, blue]) => Color.fromRgb(red, green, blue)));

export const shadeColorAt = (position: number) => shade.colorAt(position);
export const glowColorAt = (position: number) => glow.colorAt(position);
