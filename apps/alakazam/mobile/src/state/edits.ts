import { BehaviorSubject } from 'rxjs';
import { distinctUntilChanged, map } from 'rxjs/operators';
import {
  BLEMISH_TOOL_KEY,
  CROP_TOOL_KEY,
  OBJECT_REMOVAL_TOOL_KEY,
  RED_EYE_TOOL_KEY,
  SKIN_EVEN_OUT_KEY,
  SKIN_SMOOTH_KEY,
  SKIN_TAN_KEY,
  SKIN_TONE_KEY,
} from '@/src/lib/project/edit-sections/keys';
import { isFaceControlKey, isPersonControlKey } from '@/src/state/control-traits';
import type { AdjustmentValue } from '@/src/lib/project/edit-sections';

// #region: Edit controls

export const adjustments = new BehaviorSubject<Record<string, AdjustmentValue>>({});
export const adjustments$ = adjustments.asObservable();
export const appliedActions = new BehaviorSubject<string[]>([]);
export const appliedActions$ = appliedActions.asObservable();
export const activeSectionKey = new BehaviorSubject('section-colorization');
export const activeSectionKey$ = activeSectionKey.asObservable();
export const focusedControlKey = new BehaviorSubject<string | null>(null);
export const focusedControlKey$ = focusedControlKey.asObservable();
/** The slider a tabulated control has open (for example Color 2 of Color Splash), so the gesture bar can act on it. */
export const openSliderState = new BehaviorSubject<{ key: string; index: number; extra?: string }>({
  key: '',
  index: 0,
});
export const openSliderState$ = openSliderState.asObservable();
/** True while a slider is being dragged, so drawn areas are hidden and come back on release. */
export const areaOutlinesHidden = new BehaviorSubject(false);
export const areaOutlinesHidden$ = areaOutlinesHidden.asObservable();
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
export const isObjectRemovalFocused$ = focusedControlKey$.pipe(
  map(key => key === OBJECT_REMOVAL_TOOL_KEY),
  distinctUntilChanged(),
);
export const isRedEyeToolFocused = () => focusedControlKey.value === RED_EYE_TOOL_KEY;
export const isCropToolFocused = () => focusedControlKey.value === CROP_TOOL_KEY;
export const isCropToolFocused$ = focusedControlKey$.pipe(
  map(key => key === CROP_TOOL_KEY),
  distinctUntilChanged(),
);
/** Controls whose work is confined to a face, so opening them can bring that face closer. */
export const isFaceControlFocused$ = focusedControlKey$.pipe(
  map(key => key !== null && isFaceControlKey(key)),
  distinctUntilChanged(),
);
export const isPersonControlFocused$ = focusedControlKey$.pipe(
  map(key => key !== null && isPersonControlKey(key)),
  distinctUntilChanged(),
);
/** Skin tools can offer the optional skin-detection models; other person controls have their own model requirements. */
const SKIN_CONTROL_KEYS = new Set([SKIN_EVEN_OUT_KEY, SKIN_SMOOTH_KEY, SKIN_TAN_KEY, SKIN_TONE_KEY]);
export const isSkinControlFocused$ = focusedControlKey$.pipe(
  map(key => key !== null && SKIN_CONTROL_KEYS.has(key)),
  distinctUntilChanged(),
);
export const wasBlemishToolFocused = new BehaviorSubject(false);
export const wasBlemishToolFocused$ = wasBlemishToolFocused.asObservable();

// #endregion
