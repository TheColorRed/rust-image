import { type MaskLike } from '@abra/core/mask';
import { type PersonDetection } from '@alakazam/mobile';
import { BehaviorSubject } from 'rxjs';

// What the editor knows about the open photo, found by the AI models. They are native masks at the size of the photo they were
// found in. This module only holds them: `lib/analysis` finds the masks that the edits use.

/** The skin in the open photo, or null until the skin model has run. */
export const skinMask = new BehaviorSubject<MaskLike | null>(null);
export const skinMask$ = skinMask.asObservable();

/** The people in the open photo, or null until person detection has run. An empty list means it found nobody. */
export const people = new BehaviorSubject<PersonDetection[] | null>(null);
export const people$ = people.asObservable();

/** Counts what a background analysis has made (see `preload` of a control), so the editor draws again when it changes. */
export const analysisVersion = new BehaviorSubject(0);
export const analysisVersion$ = analysisVersion.asObservable();

/**
 * The square around each person's face, by the person's id, in the pixels of the photo the people were found in, or null until the
 * face models have run. A person whose face was not found has none.
 */
export const faceBoxes = new BehaviorSubject<Record<number, { x: number; y: number; width: number; height: number }> | null>(null);
export const faceBoxes$ = faceBoxes.asObservable();
