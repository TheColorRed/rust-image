import { BehaviorSubject } from 'rxjs';
import { distinctUntilChanged, map } from 'rxjs/operators';
import {
  BLEMISH_TOOL_KEY,
  BROW_BONE_KEY,
  EYELINER_KEY,
  EYESHADOW_KEY,
  LIPSTICK_KEY,
  HAIR_COLOR_KEY,
  RED_EYE_TOOL_KEY,
  SKIN_EVEN_OUT_KEY,
  SKIN_SMOOTH_KEY,
  SKIN_TAN_KEY,
  SKIN_TONE_KEY,
  UNDER_EYE_KEY,
  BLUSH_KEY,
  LIP_LINER_KEY,
  HIGHLIGHTER_KEY,
  CONTOUR_KEY,
  TEETH_WHITENING_KEY,
} from '@/src/lib/edit-sections/keys';
import type { AdjustmentValue } from '@/src/lib/edit-sections';

// #region: Edit controls

export const adjustments = new BehaviorSubject<Record<string, AdjustmentValue>>({});
export const adjustments$ = adjustments.asObservable();
export const appliedActions = new BehaviorSubject<string[]>([]);
export const appliedActions$ = appliedActions.asObservable();
export const activeSectionKey = new BehaviorSubject('section-colorization');
export const activeSectionKey$ = activeSectionKey.asObservable();
export const focusedControlKey = new BehaviorSubject<string | null>(null);
export const focusedControlKey$ = focusedControlKey.asObservable();
/** The group whose controls the section's row is showing in place of its own, or null. */
export const openGroupKey = new BehaviorSubject<string | null>(null);
export const openGroupKey$ = openGroupKey.asObservable();
/** Bumped when undo or redo changes the edit state, so an open slider remounts at the restored value. */
export const historyVersion = new BehaviorSubject(0);
export const historyVersion$ = historyVersion.asObservable();

// #endregion

// #region: Derived focus state

export const isBlemishToolFocused = () => focusedControlKey.value === BLEMISH_TOOL_KEY;
export const isBlemishToolFocused$ = focusedControlKey$.pipe(
  map(key => key === BLEMISH_TOOL_KEY),
  distinctUntilChanged(),
);
export const isRedEyeToolFocused = () => focusedControlKey.value === RED_EYE_TOOL_KEY;
export const isPersonControlFocused$ = focusedControlKey$.pipe(
  map(
    key =>
      key === SKIN_EVEN_OUT_KEY ||
      key === SKIN_SMOOTH_KEY ||
      key === SKIN_TAN_KEY ||
      key === SKIN_TONE_KEY ||
      key === LIPSTICK_KEY ||
      key === EYELINER_KEY ||
      key === EYESHADOW_KEY ||
      key === BROW_BONE_KEY ||
      key === UNDER_EYE_KEY ||
      key === BLUSH_KEY ||
      key === LIP_LINER_KEY ||
      key === HIGHLIGHTER_KEY ||
      key === CONTOUR_KEY ||
      key === TEETH_WHITENING_KEY ||
      key === HAIR_COLOR_KEY,
  ),
  distinctUntilChanged(),
);
export const wasBlemishToolFocused = new BehaviorSubject(false);
export const wasBlemishToolFocused$ = wasBlemishToolFocused.asObservable();

// #endregion
