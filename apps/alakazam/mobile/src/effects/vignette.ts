import { blend } from '@abra/core/blend';
import { Color } from '@abra/core/color';
import { ColorStop, Gradient, radialGradient } from '@abra/core/gradient';
import { ImageLike } from '@abra/core/image';

/**
 * Darkens the edges of `photo` in place. `strength` is 0 (nothing) to 1 (black corners).
 */
export const vignette = (photo: ImageLike, strength: number, distance: number = 0.5): void => {
  const clear = Color.transparent();
  const dark = Color.fromRgba(0, 0, 0, Math.round(255 * strength));
  // Clear in the middle, darkening towards the oval that passes through the corners.
  const gradient = new Gradient([new ColorStop(clear, 0), new ColorStop(clear, distance), new ColorStop(dark, 1)]);
  const width = photo.width();
  const height = photo.height();
  const cx = width / 2;
  const cy = height / 2;
  const edges = radialGradient(width, height, gradient, cx, cy, cx * Math.SQRT2, cy * Math.SQRT2);
  // As the library writes it: blend the edges onto the photo.
  blend(edges).apply(photo);
};
