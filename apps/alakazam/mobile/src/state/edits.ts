import { BehaviorSubject, combineLatest } from 'rxjs';
import { distinctUntilChanged, map } from 'rxjs/operators';
import {
  BLEMISH_TOOL_KEY,
  CROP_TOOL_KEY,
  GRADIENT_SETTINGS_KEY,
  STRIPE_KEY,
  RED_EYE_TOOL_KEY,
  SKIN_EVEN_OUT_KEY,
  SKIN_SMOOTH_KEY,
  SKIN_TAN_KEY,
  SKIN_TONE_KEY,
} from '@/src/lib/project/edit-sections/keys';
import { isFaceControlKey, isPersonControlKey } from '@/src/state/control-traits';
import { objectRemovalToolKey$ } from '@/src/state/plugin-tool-panels';
import type { AdjustmentValue, PickerItem } from '@/src/lib/project/edit-sections';

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
export const isObjectRemovalFocused$ = combineLatest([focusedControlKey$, objectRemovalToolKey$]).pipe(
  map(([focused, tool]) => tool !== null && focused === tool),
  distinctUntilChanged(),
);
export const isRedEyeToolFocused = () => focusedControlKey.value === RED_EYE_TOOL_KEY;
export const isCropToolFocused = () => focusedControlKey.value === CROP_TOOL_KEY;
export const isCropToolFocused$ = focusedControlKey$.pipe(
  map(key => key === CROP_TOOL_KEY),
  distinctUntilChanged(),
);
/**
 * The picker item whose sub-menu is open (the color schemes of the gradient's From photo card), or `null`. Its items are shown in place of the
 * picker's, its label is the last crumb of the breadcrumbs, and the back button closes it, as it steps out of any other sub-menu.
 */
export const pickerSubmenu = new BehaviorSubject<PickerItem | null>(null);
export const pickerSubmenu$ = pickerSubmenu.asObservable();

/** Whether the gradient overlay is open, which is when its two handles show on the photo. */
export const isGradientFocused$ = focusedControlKey$.pipe(
  map(key => key === GRADIENT_SETTINGS_KEY),
  distinctUntilChanged(),
);
/**
 * Whether the stripes overlay is open, which is when its text dialogs can be asked for. It opens as a row of tabs (as Bokeh does), so it is
 * open while its row shows or while any of its tabs (the settings, the colors, the sub alignment) is.
 */
export const isStripeFocused$ = combineLatest([focusedControlKey$, openGroupKey$]).pipe(
  map(([key, group]) => (group?.startsWith(STRIPE_KEY) ?? false) || (key?.startsWith(STRIPE_KEY) ?? false)),
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
