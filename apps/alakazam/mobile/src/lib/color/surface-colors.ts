/**
 * The colors the native picker surfaces (the curve plot, the gradient bar, the color square) are drawn in, each as red, green, blue and alpha from 0 to 255.
 * They are drawn by the library, not by React, so they do not follow the theme; like the color bar picker they are a fixed set that reads on both themes.
 */

/** The curve plot: the background, the grid, the line, a point and the selected point. */
export const CURVE_SURFACE_COLORS = [
  [38, 38, 44, 255],
  [78, 78, 88, 255],
  [0, 122, 255, 255],
  [0, 122, 255, 255],
  [255, 149, 0, 255],
];

/** The gradient bar: the ring round the selected stop, and the outline of the bar. */
export const GRADIENT_SURFACE_ACCENT = [255, 149, 0, 255];
export const GRADIENT_SURFACE_OUTLINE = [120, 120, 130, 255];

/** The color square: the ring of its marks, and the halo under it. */
export const COLOR_SURFACE_RING = [255, 255, 255, 255];
export const COLOR_SURFACE_SHADOW = [0, 0, 0, 200];
