import { BehaviorSubject, combineLatest } from 'rxjs';
import { distinctUntilChanged, map } from 'rxjs/operators';
import {
  BLEMISH_TOOL_KEY,
  SKIN_SMOOTH_KEY,
  SKIN_TAN_KEY,
  SKIN_TONE_KEY,
  findSection,
  type EditControl,
} from '@/src/lib/edit-sections';

// #region: Edit controls

export const adjustments = new BehaviorSubject<Record<string, number>>({});
export const adjustments$ = adjustments.asObservable();
export const appliedActions = new BehaviorSubject<string[]>([]);
export const appliedActions$ = appliedActions.asObservable();
export const activeSectionKey = new BehaviorSubject('section-colorization');
export const activeSectionKey$ = activeSectionKey.asObservable();
export const focusedControlKey = new BehaviorSubject<string | null>(null);
export const focusedControlKey$ = focusedControlKey.asObservable();
/** Bumped on reset so sliders remount at their default value. */
export const resetVersion = new BehaviorSubject(0);
export const resetVersion$ = resetVersion.asObservable();

// #endregion

// #region: Derived focus state

export const getFocusedControl = (): EditControl | undefined => {
  const key = focusedControlKey.value;
  return key ? findSection(activeSectionKey.value).controls.find(control => control.key === key) : undefined;
};
export const isBlemishToolFocused = () => getFocusedControl()?.key === BLEMISH_TOOL_KEY;
export const focusedControl$ = combineLatest([activeSectionKey$, focusedControlKey$]).pipe(
  map(([sectionKey, key]) => (key ? findSection(sectionKey).controls.find(control => control.key === key) : undefined)),
  distinctUntilChanged(),
);
export const isSkinControlFocused$ = focusedControl$.pipe(
  map(
    control =>
      control?.kind === 'slider' &&
      (control.key === SKIN_SMOOTH_KEY || control.key === SKIN_TAN_KEY || control.key === SKIN_TONE_KEY),
  ),
  distinctUntilChanged(),
);
export const isBlemishToolFocused$ = focusedControl$.pipe(
  map(control => control?.key === BLEMISH_TOOL_KEY),
  distinctUntilChanged(),
);
export const wasBlemishToolFocused = new BehaviorSubject(false);
export const wasBlemishToolFocused$ = wasBlemishToolFocused.asObservable();

// #endregion
