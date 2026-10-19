import { grayscale } from '@abra/core/adjustments';
import { blend } from '@abra/core/blend';
import { Color } from '@abra/core/color';
import { Area } from '@abra/core/geometry';
import { gaussianBlur } from '@abra/core/filters';
import { Mask, type MaskLike } from '@abra/core/mask';
import { type AbraImageLike } from '@alakazam/mobile';
import { owning } from '@/src/lib/native-handles';

/** How far the edge of the color fades, as a fraction of the shorter side of the photo. */
const EDGE_FADE = 0.008;

/** One color to keep: its hue in degrees, and the area it is kept in as rings of `[x, y]` fractions of the photo (0 to 1). No area means the whole photo. */
export interface SplashColor {
  hue: number;
  area?: number[][][];
}

/**
 * The mask of an area: every counter-clockwise ring is filled, and every clockwise ring is a hole cut out of that fill. Rings come
 * from the boolean ops, so their winding says which they are.
 */
const areaMask = (rings: number[][][], width: number, height: number, keep: <V>(value: V) => V): MaskLike => {
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
 * Keeps the colors in `colors` in `photo` and turns everything else black and white. A color is kept only inside its own area,
 * so a matching color outside that area turns gray. `range` is how many degrees of hue on each side of a hue count as that color.
 * With no colors the whole photo turns black and white. The edge between the colors and the gray fades out so it does not cut
 * hard. Changes `photo` in place.
 */
export const colorSplash = (photo: AbraImageLike, colors: SplashColor[], range: number): void =>
  owning((keep) => {
    const width = photo.width();
    const height = photo.height();
    const colored = keep(photo.clone());
    let chosen: MaskLike | undefined;
    // The union of every color's area. Unset when a color has no area, which keeps the whole photo in reach.
    let bounds: MaskLike | undefined;
    let unbounded = false;
    for (const { hue, area } of colors) {
      const color = keep(Color.fromHsv(hue, 1, 1));
      let mask: MaskLike = keep(Mask.fromColorRange(photo, color, range, 0.25, 0.1, 0.15));
      if (area) {
        const region = keep(areaMask(area, width, height, keep));
        mask = keep(mask.multiplied(region));
        bounds = bounds ? keep(bounds.maximum(region)) : region;
      } else {
        unbounded = true;
      }
      chosen = chosen ? keep(chosen.maximum(mask)) : mask;
    }
    keep(grayscale()).apply(photo);
    if (!chosen) return;
    // The fade is applied first and the outline is applied after it, so the blur cannot spread a color past the outline.
    const edge = keep(chosen.toImage());
    keep(gaussianBlur(Math.max(2, Math.min(photo.width(), photo.height()) * EDGE_FADE))).apply(edge);
    let faded: MaskLike = keep(Mask.fromImage(edge));
    if (!unbounded && bounds) faded = keep(faded.multiplied(bounds));
    keep(faded).applyToImage(colored);
    keep(blend(colored)).apply(photo);
  });
