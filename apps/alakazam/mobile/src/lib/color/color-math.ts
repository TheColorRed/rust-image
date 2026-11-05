/**
 * The small sums a color picker does while a finger is dragging: red, green and blue (each 0 to 255) from and to hue, colorfulness and
 * brightness, and hex text. They are plain numbers, not images, so they run on the JavaScript thread at every move of a drag without a
 * native call; anything that touches pixels is the library's.
 */

/** Red, green and blue, each 0 to 255. */
export type Rgb = [number, number, number];

/**
 * A color as a person picks it: which color it is (`hue`, 0 to 360 degrees round the color wheel), how colorful it is (`saturation`, 0 to 1, a
 * grey at 0), and how bright it is (`brightness`, 0 to 1, black at 0). The library's names are used here; the picker shows the plain words.
 */
export type Hsb = { hue: number; saturation: number; brightness: number };

export const clamp = (value: number, low: number, high: number) => Math.min(Math.max(value, low), high);

/** The red, green and blue of a hue, colorfulness and brightness. */
export const hsbToRgb = ({ hue, saturation, brightness }: Hsb): Rgb => {
  const h = (((hue % 360) + 360) % 360) / 60;
  const chroma = brightness * saturation;
  const second = chroma * (1 - Math.abs((h % 2) - 1));
  const base = brightness - chroma;
  const sector = Math.floor(h);
  const [r, g, b] = [
    [chroma, second, 0],
    [second, chroma, 0],
    [0, chroma, second],
    [0, second, chroma],
    [second, 0, chroma],
    [chroma, 0, second],
  ][sector % 6];
  return [r, g, b].map(channel => Math.round((channel + base) * 255)) as Rgb;
};

/**
 * The hue, colorfulness and brightness of a red, green and blue. A grey has no hue and black has no colorfulness, so `fallback` is what the
 * hue (or the colorfulness for black) is kept at, which stops a picker's hue jumping to red as the square is dragged to grey or black.
 */
export const rgbToHsb = ([red, green, blue]: Rgb, fallback: Hsb = { hue: 0, saturation: 0, brightness: 0 }): Hsb => {
  const [r, g, b] = [red, green, blue].map(channel => clamp(channel, 0, 255) / 255);
  const high = Math.max(r, g, b);
  const range = high - Math.min(r, g, b);
  const brightness = high;
  const saturation = high === 0 ? fallback.saturation : range / high;
  if (range === 0) return { hue: fallback.hue, saturation, brightness };
  const sector = high === r ? ((g - b) / range + 6) % 6 : high === g ? (b - r) / range + 2 : (r - g) / range + 4;
  return { hue: sector * 60, saturation, brightness };
};

/** `#rrggbb`, in lower case. */
export const rgbToHex = ([r, g, b]: Rgb) =>
  `#${[r, g, b].map(channel => clamp(Math.round(channel), 0, 255).toString(16).padStart(2, '0')).join('')}`;

/** Red, green and blue from text such as `3399cc`, `#3399cc` or `#39c`, or nothing when it is not a color. */
export const hexToRgb = (text: string): Rgb | null => {
  const digits = text.trim().replace(/^#/, '');
  const full = /^[0-9a-f]{3}$/i.test(digits) ? [...digits].map(digit => digit + digit).join('') : digits;
  if (!/^[0-9a-f]{6}$/i.test(full)) return null;
  return [0, 2, 4].map(at => parseInt(full.slice(at, at + 2), 16)) as Rgb;
};

/** Black or white, whichever shows better over this color, for text on a swatch. */
export const readableOn = ([r, g, b]: Rgb) => ((r * 299 + g * 587 + b * 114) / 1000 > 150 ? '#000000' : '#ffffff');
