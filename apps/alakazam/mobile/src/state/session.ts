import { AbraImage } from '@alakazam/mobile';
import { BehaviorSubject } from 'rxjs';
import { type EditablePhoto } from '@/src/lib/edit-source';

// #region: Source and native image session

export const photo = new BehaviorSubject<EditablePhoto | null>(null);
export const photo$ = photo.asObservable();
export const originalImage = new BehaviorSubject<AbraImage | null>(null);
export const originalImage$ = originalImage.asObservable();
export const currentImage = new BehaviorSubject<AbraImage | null>(null);
export const currentImage$ = currentImage.asObservable();
export const editBaseImage = new BehaviorSubject<AbraImage | null>(null);
export const editBaseImage$ = editBaseImage.asObservable();

// #endregion

// #region: Session status

export const ready = new BehaviorSubject(false);
export const ready$ = ready.asObservable();
export const loadError = new BehaviorSubject<string | null>(null);
export const loadError$ = loadError.asObservable();
export const busy = new BehaviorSubject(false);
export const busy$ = busy.asObservable();
/**
 * Set when the pending edit is already showing on screen through a live preview, so its replay runs unseen and needs no
 * busy indicator. The replay clears it.
 */
export const replayHidden = new BehaviorSubject(false);

// #endregion
