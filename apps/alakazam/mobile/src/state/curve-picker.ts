import { BehaviorSubject } from 'rxjs';

/** A curve as it is edited: points `[x, y]`, with `x` the place along it from 0 (the start) to 1 (the end) and `y` the value, from the editor's `min` to `max`. Never empty. */
export type CurveValue = [number, number][];

/** A named shape the editor offers to start from. */
export type CurvePreset = { label: string; value: CurveValue };

export type PickCurveOptions = {
  /** The words over the editor. Defaults to "Edit the curve". */
  title?: string;
  /** The curve the editor opens on. */
  value: CurveValue;
  /** The lowest and highest value the editor shows and a point can have. */
  min: number;
  max: number;
  /** Shapes to start from, shown above the curve. */
  presets?: CurvePreset[];
  /** Called as the curve changes, for a caller that shows the change at once. `pickCurve` itself answers when they press OK. */
  onChange?: (value: CurveValue) => void;
};

/** An open request for the editor, which the host (`CurvePickerHost`) shows and answers. `id` is new for each request. */
export type CurvePickerRequest = { id: number; options: PickCurveOptions; resolve: (result: CurveValue | null) => void };

let nextRequestId = 0;

/** The request the editor is open for, or nothing when it is closed. */
export const curvePickerRequest = new BehaviorSubject<CurvePickerRequest | null>(null);
export const curvePickerRequest$ = curvePickerRequest.asObservable();

/** Opens the curve editor and answers with the curve the person made on OK, or `null` on Cancel. Asking while it is open cancels the one before. */
export const pickCurve = (options: PickCurveOptions): Promise<CurveValue | null> =>
  new Promise(resolve => {
    curvePickerRequest.value?.resolve(null);
    curvePickerRequest.next({ id: nextRequestId++, options, resolve });
  });

/** Closes the editor and answers the request: the curve, or `null` for cancel. */
export const finishCurvePick = (result: CurveValue | null) => {
  const request = curvePickerRequest.value;
  curvePickerRequest.next(null);
  request?.resolve(result);
};
