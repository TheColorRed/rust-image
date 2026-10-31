import type { DownloadedFont } from '@alakazam/mobile';
import { BehaviorSubject } from 'rxjs';

/** One family of the Google Fonts list, as read by `lib/fonts/google-fonts`. */
export type GoogleFont = {
  family: string;
  /** `sans-serif`, `serif`, `display`, `handwriting` or `monospace`. */
  category: string;
  /** The URL of each style's font file, by style (`regular`, `italic`, `700`, `700italic`...). */
  files: Record<string, string>;
};

/**
 * The Google Fonts list from the last read, kept here so the fonts screen shows it as soon as it opens and then refreshes it.
 * Empty until it has been read once in this run of the app.
 */
export const googleFonts = new BehaviorSubject<GoogleFont[]>([]);
export const googleFonts$ = googleFonts.asObservable();

/** The font families on this device, brought up to date after a download or a delete. */
export const installedFonts = new BehaviorSubject<DownloadedFont[]>([]);
export const installedFonts$ = installedFonts.asObservable();
