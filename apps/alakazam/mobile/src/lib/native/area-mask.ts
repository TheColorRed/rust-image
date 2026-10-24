import { Color } from '@abra/core/color';
import { Area, polygonDifference, polygonUnion } from '@abra/core/geometry';
import { Mask, type MaskLike } from '@abra/core/mask';
import type { DrawMode } from '@/src/state/draw-area';

/**
 * The mask of an area: every counter-clockwise ring is filled, and every clockwise ring is a hole cut out of that fill. Rings come
 * from the boolean ops, so their winding says which they are.
 * `keep` holds every native object made so the caller can free them.
 */
export const areaMask = (rings: number[][][], width: number, height: number, keep: <V>(value: V) => V): MaskLike => {
  let fill: MaskLike | undefined;
  let holes: MaskLike | undefined;
  for (const ring of rings) {
    const outline = keep(Area.fromPoints(ring.map(([x, y]) => [x * width, y * height])));
    const part = keep(Mask.empty(width, height));
    part.drawArea(outline, keep(Color.white()), undefined);
    const winding = ring.reduce((sum, [x, y], index) => {
      const [nextX, nextY] = ring[(index + 1) % ring.length];
      return sum + x * nextY - nextX * y;
    }, 0);
    if (winding >= 0) fill = fill ? keep(fill.maximum(part)) : part;
    else holes = holes ? keep(holes.maximum(part)) : part;
  }
  if (!fill) return keep(Mask.empty(width, height));
  return holes ? keep(fill.subtracted(holes)) : fill;
};

/**
 * An area, as rings of `[x, y]` fractions of the photo, changed by a drawn loop: the loop starts the area over (`new`), is added to it
 * (`add`) or is taken away from it (`subtract`). A loop is in the photo's pixels, as `DrawAreaOverlay` gives it. Taking away from no
 * area takes it from the whole photo.
 */
export function combineArea(
  area: readonly number[][][] | undefined,
  loop: readonly { x: number; y: number }[],
  mode: DrawMode,
  image: { width: number; height: number },
): number[][][] {
  const { width, height } = image;
  const current = area?.map(ring => ring.map(([x, y]) => ({ x: x * width, y: y * height })));
  const whole = [{ x: 0, y: 0 }, { x: width, y: 0 }, { x: width, y: height }, { x: 0, y: height }];
  const rings =
    mode === 'subtract'
      ? polygonDifference(current ?? [whole], [[...loop]])
      : polygonUnion(mode === 'add' ? (current ?? []) : [], [[...loop]]);
  return rings.map(ring => ring.map(point => [point.x / width, point.y / height]));
}
