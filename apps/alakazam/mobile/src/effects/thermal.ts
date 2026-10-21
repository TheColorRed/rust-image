import { autoTone, gradientMap } from '@abra/core/adjustments';
import { blend } from '@abra/core/blend';
import { Color } from '@abra/core/color';
import { ColorStop, Gradient } from '@abra/core/gradient';
import { type AbraImageLike } from '@alakazam/mobile';
import { owning } from '@/src/lib/native/native-handles';

/** The heat colors from cold to hot, each at its place from 0 to 1 along the gradient. */
const HEAT: [number, [number, number, number]][] = [
  [0, [0, 0, 160]],
  [0.25, [0, 200, 255]],
  [0.5, [255, 255, 0]],
  [0.75, [255, 120, 0]],
  [1, [255, 0, 0]],
];

/**
 * Turns `photo` into a thermal heat map in place: its brightness is stretched to use the whole range, then mapped to a gradient
 * from blue in the darks, through cyan and yellow, to red in the lights. At 0 the photo is unchanged and at 1 it is fully thermal.
 * Values in between fade from one to the other.
 */
export const thermal = (photo: AbraImageLike, amount: number): void =>
  owning(keep => {
    if (amount <= 0) return;
    const stops = HEAT.map(([place, rgb]) => keep(new ColorStop(keep(Color.fromRgb(...rgb)), place)));
    const gradient = keep(new Gradient(stops));
    const map = keep(photo.clone());
    keep(autoTone()).apply(map);
    keep(gradientMap(gradient)).apply(map);
    keep(keep(blend(map)).withOpacity(amount)).apply(photo);
  });
