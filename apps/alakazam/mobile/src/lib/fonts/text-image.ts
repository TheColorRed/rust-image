import { release } from '@/src/lib/native/native-handles';
import { Color, type ColorLike } from '@abra/core/color';
import { Fill } from '@abra/core/fill';
import type { ImageLike } from '@abra/core/image';
import { Font, TextAlign, TextSize, WordWrap, type FontLike } from '@abra/core/typography';
import { downloadedFonts } from '@alakazam/mobile';

/**
 * What to write. The editor and the plugins both describe text this way and get an image back, so a font is found, loaded and set the same
 * way everywhere. Everything is the library's `Text`; this only finds the font file of a downloaded family.
 */
export type TextSpec = {
  text: string;
  /** A family downloaded on the Fonts screen. */
  family: string;
  /** The size of the letters, in pixels. */
  size: number;
  color?: ColorLike;
  /** Wraps the text to this many pixels and lines it up as `align` says. Without it the text is one line per line break. */
  width?: number;
  align?: 'left' | 'center' | 'right';
};

/** The families on this device and the path of each one's regular file (or its first, when it has no regular). Works offline. */
export const installedFamilies = (): { family: string; path: string }[] =>
  downloadedFonts().map(({ family, files }) => ({
    family,
    path: files.find(file => /[\\/]regular\.(ttf|otf)$/i.test(file)) ?? files[0],
  }));

/** The fonts loaded so far, by file, so a family is read from its file once however often text is set in it. */
const loaded = new Map<string, FontLike>();

const fontOf = (family: string): FontLike | null => {
  const path = installedFamilies().find(each => each.family === family)?.path;
  if (!path) return null;
  if (!loaded.has(path)) loaded.set(path, Font.fromPath(path));
  return loaded.get(path)!;
};

/** Lets go of the loaded fonts, for when the screen using them closes. They are loaded again when next used. */
export const releaseFonts = () => {
  release(...loaded.values());
  loaded.clear();
};

const ALIGN = { left: TextAlign.Left, center: TextAlign.Center, right: TextAlign.Right };

/**
 * The text as a new transparent image sized around the letters, for the caller to own and release. `null` when there is nothing to write:
 * no text, or a family that is not on this device (deleted since it was chosen).
 */
export const renderText = (spec: TextSpec): ImageLike | null => {
  const font = spec.text.trim() === '' ? null : fontOf(spec.family);
  if (!font) return null;
  const size = TextSize.Pixels.new(Math.max(1, spec.size));
  const color = spec.color ?? Color.black();
  const fill = Fill.solid(color);
  const steps = [font.text(spec.text)];
  const next = (made: (typeof steps)[number]) => steps.push(made);
  next(steps[steps.length - 1].withSize(size));
  next(steps[steps.length - 1].withFill(fill));
  if (spec.width) {
    next(steps[steps.length - 1].withWidth(spec.width));
    next(steps[steps.length - 1].withWordWrap(WordWrap.Normal));
  }
  next(steps[steps.length - 1].withAlignment(ALIGN[spec.align ?? 'left']));
  const image = steps[steps.length - 1].toImage();
  release(...steps, fill, spec.color ? undefined : color);
  return image;
};
