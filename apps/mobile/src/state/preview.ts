import { AbraImage } from '@alakazam/mobile';
import { type SkImage } from '@shopify/react-native-skia';
import { BehaviorSubject } from 'rxjs';

// #region: Preview rendering

export const preview = new BehaviorSubject<SkImage | null>(null);
export const preview$ = preview.asObservable();
/** A frame rendered live while a slider is dragged; shown over the preview until the committed edit replaces it. */
export const liveFrame = new BehaviorSubject<SkImage | null>(null);
export const liveFrame$ = liveFrame.asObservable();
/** Pixel size of the frames a live preview draws on the native surface, or null when none is. */
export const liveSurface = new BehaviorSubject<{ width: number; height: number } | null>(null);
export const liveSurface$ = liveSurface.asObservable();
export const previewSourceImage = new BehaviorSubject<AbraImage | null>(null);
export const previewSourceImage$ = previewSourceImage.asObservable();
export const showingCheckpoint = new BehaviorSubject(false);
export const showingCheckpoint$ = showingCheckpoint.asObservable();

// #endregion

// #region: Control thumbnails

/** PNG data URIs keyed by control key; shown with a plain `Image` (a Skia Canvas per thumbnail crashed). */
export const controlThumbnails = new BehaviorSubject<Record<string, string>>({});
export const controlThumbnails$ = controlThumbnails.asObservable();

// #endregion

// #region: Safe-area insets

export const topInset = new BehaviorSubject(0);
export const topInset$ = topInset.asObservable();
export const bottomInset = new BehaviorSubject(0);
export const bottomInset$ = bottomInset.asObservable();

// #endregion
