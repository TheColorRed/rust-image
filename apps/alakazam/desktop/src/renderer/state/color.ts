import { BehaviorSubject, combineLatest, map, of, switchMap } from 'rxjs';

export type ActiveColor = 'fg' | 'bg';

/** The foreground color used for drawing operations within the application. */
export const foregroundColor = new BehaviorSubject('#ffffff');
/** The background color used for drawing operations within the application. */
export const backgroundColor = new BehaviorSubject('#000000');
/** The hue component of the current color in HSL color space (0-360). */
export const hue = new BehaviorSubject<string>('#ff0000');
/** The currently active color, either 'foreground' or 'background'. */
export const activeColorState = new BehaviorSubject<ActiveColor>('fg');

/** An observable that emits the current foreground color. */
export const foregroundColor$ = foregroundColor.pipe(
  switchMap(fg =>
    of(fg).pipe(
      switchMap(color => window.gizmos.colorPicker.getHueFrom(color)),
      map(hueInfo => ({ color: fg, hue: hueInfo })),
    ),
  ),
);
/** An observable that emits the current background color. */
export const backgroundColor$ = backgroundColor.pipe(
  switchMap(bg =>
    of(bg).pipe(
      switchMap(color => window.gizmos.colorPicker.getHueFrom(color)),
      map(hueInfo => ({ color: bg, hue: hueInfo })),
    ),
  ),
);
/** An observable that emits the current hue value. */
export const hue$ = hue.pipe(
  switchMap(hueValue => of(hueValue).pipe(switchMap(color => window.gizmos.colorPicker.getHueFrom(color)))),
);
/** An observable that emits an object containing both the foreground and background colors. */
export const colors$ = combineLatest({ fg: foregroundColor$, bg: backgroundColor$, hue: hue$ });
/** An observable that emits the currently active color value. */
export const activeColorState$ = activeColorState.asObservable();
/**
 * Sets the currently active color state (foreground or background) to the specified color.
 * @param color The color to set as the active color.
 */
export function setTheActiveColor(color: string) {
  const active = activeColorState.getValue();
  active === 'fg' ? foregroundColor.next(color) : backgroundColor.next(color);
}
