import { Color, ColorStop, Gradient, ImageLike, blend, radialGradient } from '@abra/core';

/**
 * Darkens the edges of `photo`. `strength` is 0 (nothing) to 1 (black corners). Returns a new image; `photo` is not changed.
 */
export const vignette = (photo: ImageLike, strength: number, distance: number = 0.5): ImageLike => {
  const clear = Color.transparent();
  const dark = Color.fromRgba(0, 0, 0, Math.round(255 * strength));
  // Clear in the middle, darkening towards the oval that passes through the corners.
  const gradient = new Gradient([new ColorStop(clear, 0), new ColorStop(clear, distance), new ColorStop(dark, 1)]);
  const cx = photo.width() / 2;
  const cy = photo.height() / 2;
  const edges = radialGradient(photo.width(), photo.height(), gradient, cx, cy, cx * Math.SQRT2, cy * Math.SQRT2);
  return blend(photo, edges, 'normal', 1);
};
