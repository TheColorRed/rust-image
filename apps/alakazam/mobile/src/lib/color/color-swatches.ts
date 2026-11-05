import type { ColorSwatch } from '@/src/state/color-picker';

/**
 * The colors the picker offers by name when a caller gives none. People ask for a color by its name ("red", "black"), not by where it is on a
 * square, so each is one tap. A new color is a line here.
 */
export const DEFAULT_SWATCHES: ColorSwatch[] = [
  { label: 'White', rgb: [255, 255, 255] },
  { label: 'Black', rgb: [0, 0, 0] },
  { label: 'Gray', rgb: [128, 128, 128] },
  { label: 'Red', rgb: [235, 40, 40] },
  { label: 'Orange', rgb: [255, 140, 30] },
  { label: 'Yellow', rgb: [255, 214, 10] },
  { label: 'Green', rgb: [60, 190, 90] },
  { label: 'Teal', rgb: [30, 190, 190] },
  { label: 'Blue', rgb: [40, 120, 255] },
  { label: 'Purple', rgb: [150, 80, 230] },
  { label: 'Pink', rgb: [255, 120, 190] },
  { label: 'Brown', rgb: [130, 85, 50] },
];
