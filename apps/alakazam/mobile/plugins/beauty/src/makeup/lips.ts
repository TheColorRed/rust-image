import { BlendMode } from '@abra/core/blend';
import { swatchesOf } from '@alakazam/sdk';

/** The colors the lips offer, each once. `lipSwatches` below turns them into the lipstick and lip liner swatches. */
const lipColors = [
  { label: 'Black', color: '#000000' },
  { label: 'White', color: '#FFFFFF' },
  { label: 'Creamy nude', color: '#EBAD81' },
  { label: 'Nude peach', color: '#F18E6A' },
  { label: 'Nude', color: '#DD7C3D' },
  { label: 'Caramel nude', color: '#B87A74' },
  { label: 'Dark nude', color: '#D27754' },
  { label: 'Coffee nude', color: '#B46F4B' },
  { label: 'Pale pink', color: '#F096BD' },
  { label: 'Nude pink', color: '#E9A8C6' },
  { label: 'Pale rose pink', color: '#EC7EAA' },
  { label: 'Rose pink', color: '#DF4A86' },
  { label: 'Browny pink', color: '#C06D7E' },
  { label: 'Dark dusky pink', color: '#AE5373' },
  { label: 'Dark pink', color: '#BC316A' },
  { label: 'Deep rose pink', color: '#E32957' },
  { label: 'Bright fuchsia', color: '#E30274' },
  { label: 'Sheer coral', color: '#EE8372' },
  { label: 'Sheer strawberry pink', color: '#E74957' },
  { label: 'Coral red', color: '#E73C42' },
  { label: 'Deep mandarin', color: '#E6421A' },
  { label: 'Bright red', color: '#E0011B' },
  { label: 'Classic red', color: '#CD0215' },
  { label: 'Deep red', color: '#BF012F' },
  { label: 'Sheer plum', color: '#C58396' },
  { label: 'Golden plum', color: '#9E4776' },
  { label: 'Plum', color: '#A6315B' },
  { label: 'Raspberry pink', color: '#B00048' },
  { label: 'Burgundy', color: '#7F023E' },
  { label: 'Deep purple plum', color: '#8C0364' },
];

/** The lipstick and lip liner swatches. The sheer ones are lighter. */
export const lipSwatches = swatchesOf(lipColors, (hue, saturation, brightness, label) => [
  hue,
  label.startsWith('Sheer') ? 0.45 : undefined,
  undefined,
  saturation,
  brightness,
]);

/**
 * How lip color is laid over the lips: with the lips' own light and shade showing through (Color), except for black, which darkens
 * them (Multiply), and white, which covers them (Normal). The values are the lipstick's and the lip liner's: the vividness and the
 * brightness are the fourth and the fifth.
 */
export const lipBlend = ([, , , saturation, brightness]: number[]) =>
  brightness < 0.2 ? BlendMode.Multiply : saturation < 0.1 && brightness > 0.9 ? BlendMode.Normal : BlendMode.Color;
