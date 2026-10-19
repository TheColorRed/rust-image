import { type Image as AbraPixels } from '@abra/core/image';
import { Area } from '@abra/core/geometry';
import { RemoverShape, ShapePosition, Units, remover } from '@abra/core/tools';
import type { AbraImageLike } from '@alakazam/mobile';

/** Repairs the polygon selected by the Remove tool using the texture immediately around its edge. */
export function removeSelection(image: AbraImageLike, points: readonly [number, number][]) {
  if (points.length < 3) return;
  const left = Math.min(...points.map(([x]) => x));
  const right = Math.max(...points.map(([x]) => x));
  const top = Math.min(...points.map(([, y]) => y));
  const bottom = Math.max(...points.map(([, y]) => y));
  const center = { x: (left + right) / 2, y: (top + bottom) / 2 };
  const reference = Math.max(3, Math.min(right - left, bottom - top) / 2);
  const area = Area.fromPoints(points.map(([x, y]) => [x - center.x, y - center.y]));
  const photo = image.clone();
  try {
    remover(RemoverShape.Area.new(area))
      .withDistance(Units.Pixels.new(Math.max(3, Math.round(reference * 0.35))))
      .withPosition(ShapePosition.Point.new(center))
      .apply(photo);
    image.replaceWith(photo);
  } finally {
    (photo as AbraPixels).uniffiDestroy();
  }
}
