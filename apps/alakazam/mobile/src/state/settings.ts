import { warn } from '@/src/lib/log';
import type { ThemePreference } from '@/src/lib/ui/theme';
import { setWorkerStackMegabytes } from '@alakazam/mobile';
import AsyncStorage from '@react-native-async-storage/async-storage';
import { BehaviorSubject, combineLatest, skip } from 'rxjs';

/** A file format a photo can be exported in. The name is the file extension the encoder picks the format from. */
export type ExportFormat = 'webp' | 'jpg' | 'png';

/** Every export format, in the order Settings lists them. */
export const EXPORT_FORMATS: { value: ExportFormat; label: string }[] = [
  { value: 'webp', label: 'WebP' },
  { value: 'jpg', label: 'JPEG' },
  { value: 'png', label: 'PNG' },
];

/** How much detail an exported photo keeps; better looks cost a bigger file. Does nothing for PNG, which loses none. */
export type ExportQuality = number;

/** Every export quality with the encoder quality (0 to 100) it stands for, in the order Settings lists them. */
export const EXPORT_QUALITIES: { value: ExportQuality; label: string; encoder: number }[] = [
  { value: 70, label: 'Good', encoder: 70 },
  { value: 85, label: 'Better', encoder: 85 },
  { value: 100, label: 'Best', encoder: 100 },
];

export const exportQuality = new BehaviorSubject<ExportQuality>(85);
export const exportQuality$ = exportQuality.asObservable();

/** The format the share sheet gets the edited photo in. WebP is smaller than JPEG at the same looks. */
export const exportFormat = new BehaviorSubject<ExportFormat>('webp');
export const exportFormat$ = exportFormat.asObservable();

/** The user's choice of light or dark; 'system' follows the OS setting. */
export const themePreference = new BehaviorSubject<ThemePreference>('system');
export const themePreference$ = themePreference.asObservable();

/** Whether developer-only tools are visible in the app. */
export const developerModeEnabled = new BehaviorSubject(false);
export const developerModeEnabled$ = developerModeEnabled.asObservable();

/** The stack sizes, in megabytes, the developer dialog offers for the image worker threads. */
export const WORKER_STACK_CHOICES = [4, 8, 16, 32, 64];

/**
 * The stack of each image worker thread, in megabytes. The workers are made once, when the first image job starts, so a new size takes
 * effect the next time the app starts.
 */
export const workerStackMegabytes = new BehaviorSubject(16);
export const workerStackMegabytes$ = workerStackMegabytes.asObservable();

/** The stack size the workers were told to use when the app started, or null until the saved settings are read. Changing the setting does not change it. */
export const workerStackInEffect = new BehaviorSubject<number | null>(null);
export const workerStackInEffect$ = workerStackInEffect.asObservable();

const STORAGE_KEY = 'settings';

/** Reads the saved settings, then saves every change to them as it happens. A missing or unreadable value keeps its default. */
(async () => {
  try {
    const saved = JSON.parse((await AsyncStorage.getItem(STORAGE_KEY)) ?? '{}');
    if (EXPORT_FORMATS.some(option => option.value === saved.exportFormat)) exportFormat.next(saved.exportFormat);
    const legacyQualities: Record<string, number> = { good: 70, better: 85, best: 100 };
    const savedQuality = typeof saved.exportQuality === 'number' ? saved.exportQuality : legacyQualities[saved.exportQuality];
    if (typeof savedQuality === 'number' && savedQuality >= 0 && savedQuality <= 100) exportQuality.next(savedQuality);
    if (WORKER_STACK_CHOICES.includes(saved.workerStackMegabytes)) workerStackMegabytes.next(saved.workerStackMegabytes);
    if (['system', 'light', 'dark'].includes(saved.themePreference)) themePreference.next(saved.themePreference);
    if (typeof saved.developerModeEnabled === 'boolean') developerModeEnabled.next(saved.developerModeEnabled);
  } catch (error) {
    warn('settings', 'could not read the saved settings:', error);
  }
  // Tells the library before the first image job, which is when the workers are made.
  setWorkerStackMegabytes(workerStackMegabytes.value);
  workerStackInEffect.next(workerStackMegabytes.value);
  // Starts after the read, and skips the value it starts with, so the defaults never overwrite what was saved.
  combineLatest([exportFormat$, exportQuality$, themePreference$, workerStackMegabytes$, developerModeEnabled$])
    .pipe(skip(1))
    .subscribe(([format, quality, theme, stack, developerMode]) => {
      setWorkerStackMegabytes(stack);
      AsyncStorage.setItem(STORAGE_KEY, JSON.stringify({ exportFormat: format, exportQuality: quality, themePreference: theme, workerStackMegabytes: stack, developerModeEnabled: developerMode })).catch(error =>
        warn('settings', 'could not save the settings:', error),
      );
    });
})();
