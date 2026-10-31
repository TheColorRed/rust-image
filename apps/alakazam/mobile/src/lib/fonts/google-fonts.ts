import { googleFonts, installedFonts, type GoogleFont } from '@/src/state/fonts';
import { deleteFont, downloadedFonts, downloadFontFile, fontPreview } from '@alakazam/mobile';

/**
 * The key for the Google Fonts Developer API (https://developers.google.com/fonts/docs/developer_api), made in the Google Cloud console.
 * It ships inside the app, so restrict it there to the Fonts API and to this app's Android package.
 */
export const GOOGLE_FONTS_API_KEY: string = 'AIzaSyBsUFqPJDpMvgS2tDrVqwwKNnSrvYG67BE';

const GOOGLE_FONTS_URL = 'https://www.googleapis.com/webfonts/v1/webfonts';

/** Reads the Google Fonts list, most popular first, and keeps it in `googleFonts`. */
export const refreshGoogleFonts = async (): Promise<GoogleFont[]> => {
  if (GOOGLE_FONTS_API_KEY === '')
    throw new Error('Add the Google Fonts API key to src/lib/fonts/google-fonts.ts to browse fonts.');
  const query = `key=${GOOGLE_FONTS_API_KEY}&sort=popularity&fields=items(family,category,files)`;
  const response = await fetch(`${GOOGLE_FONTS_URL}?${query}`, { cache: 'no-store' });
  if (!response.ok) throw new Error(`Google Fonts could not be read (${response.status}).`);
  const items: { family?: unknown; category?: unknown; files?: Record<string, string> }[] =
    (await response.json()).items ?? [];
  const fonts = items
    .filter(item => typeof item.family === 'string' && item.files && Object.keys(item.files).length > 0)
    .map(item => ({
      family: String(item.family),
      category: typeof item.category === 'string' ? item.category : '',
      files: item.files as Record<string, string>,
    }));
  googleFonts.next(fonts);
  return fonts;
};

/** Brings the list of fonts on this device up to date. Works offline. */
export const syncInstalledFonts = () => installedFonts.next(downloadedFonts());

/**
 * Downloads every style of a family, one after another, telling `listener` how far it has got over all of them from 0 to 1. A style that
 * is already on this device is not downloaded again.
 */
export const downloadFont = async (
  font: GoogleFont,
  listener: { update: (progress: number) => void },
): Promise<void> => {
  const styles = Object.entries(font.files);
  for (const [index, [style, url]] of styles.entries()) {
    const extension = /\.otf$/i.test(url) ? 'otf' : 'ttf';
    // Google lists some files over http; the files are the same over https.
    await downloadFontFile(font.family, `${style}.${extension}`, url.replace(/^http:/, 'https:'), {
      update: share => listener.update((index + share) / styles.length),
    });
  }
  listener.update(1);
  syncInstalledFonts();
};

/** Deletes every style of a family from this device. */
export const deleteInstalledFont = (family: string) => {
  deleteFont(family);
  syncInstalledFonts();
};

/**
 * The file:// URI of a picture of the family's name in its own font, in this `#rrggbb` color. The native side makes it the first time and
 * keeps it; it downloads only one style of a family that is not on this device. Ask for it when a row comes into view, not before.
 */
export const previewUri = async (font: GoogleFont, color: string): Promise<string> => {
  const [red, green, blue] = /^#[0-9a-f]{6}$/i.test(color) ? [1, 3, 5].map(at => parseInt(color.slice(at, at + 2), 16)) : [0, 0, 0];
  // A family on this device needs no URL: the native side uses its files.
  const url = font.files.regular ?? Object.values(font.files)[0] ?? '';
  return `file://${await fontPreview(font.family, url.replace(/^http:/, 'https:'), red, green, blue)}`;
};
