import { type Image as AbraPixels } from '@abra/core/image';
import { RemoverShape, ShapePosition, Units, remover } from '@abra/core/tools';
import { type AbraImageLike } from '@alakazam/mobile';

/**
 * Removes a blemish centered at `(x, y)` (image pixels), blending in texture borrowed from nearby. `radius` is the size of the
 * healed spot itself; the remover samples a wider ring around it for context, scaled to the spot so larger blemishes still get
 * an appropriately sized sample.
 */
export function removeBlemish(image: AbraImageLike, x: number, y: number, radius: number) {
  const photo = image.clone();
  try {
    remover(RemoverShape.Circle.new(radius))
      .withDistance(Units.Pixels.new(Math.max(Math.round(radius * 0.6), 3)))
      .withPosition(ShapePosition.Point.new({ x, y }))
      .apply(photo);
    image.replaceWith(photo);
  } finally {
    // It is a native image the garbage collector cannot see the size of.
    (photo as AbraPixels).uniffiDestroy();
  }
}
