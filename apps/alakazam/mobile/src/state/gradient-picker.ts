import type { Rgb } from '@/src/lib/color/color-math';
import type { ColorSwatch } from '@/src/state/color-picker';
import { BehaviorSubject } from 'rxjs';

/**
 * A gradient as it is edited: the colors it passes through and, apart from them, how strong it is along its length, each at a place from 0
 * (the start) to 1 (the end). The strength has stops of its own, as in Unity's gradient editor, so the color and the strength do not have to
 * change in the same places. Neither list is empty.
 */
export type GradientValue = {
  colors: { at: number; color: Rgb }[];
  /** 0 (not there) to 255 (solid). */
  alphas: { at: number; alpha: number }[];
};

export type PickGradientOptions = {
  /** The words over the editor. Defaults to "Edit the gradient". */
  title?: string;
  /** The gradient the editor opens on. */
  value: GradientValue;
  /** The named colors shown in the color picker a stop opens. Defaults to the app's common set. */
  swatches?: ColorSwatch[];
  /** Called as the gradient changes, for a caller that shows the change at once. `pickGradient` itself answers when they press OK. */
  onChange?: (value: GradientValue) => void;
};

/** An open request for the editor, which the host (`GradientPickerHost`) shows and answers. `id` is new for each request. */
export type GradientPickerRequest = {
  id: number;
  options: PickGradientOptions;
  resolve: (result: GradientValue | null) => void;
};

let nextRequestId = 0;

/** The request the editor is open for, or nothing when it is closed. */
export const gradientPickerRequest = new BehaviorSubject<GradientPickerRequest | null>(null);
export const gradientPickerRequest$ = gradientPickerRequest.asObservable();

/**
 * Opens the gradient editor and answers with the gradient the person made when they press OK, or `null` when they cancel. Asking while it is
 * open cancels the one before.
 */
export const pickGradient = (options: PickGradientOptions): Promise<GradientValue | null> =>
  new Promise(resolve => {
    gradientPickerRequest.value?.resolve(null);
    gradientPickerRequest.next({ id: nextRequestId++, options, resolve });
  });

/** Closes the editor and answers the request: the gradient, or `null` for cancel. */
export const finishGradientPick = (result: GradientValue | null) => {
  const request = gradientPickerRequest.value;
  gradientPickerRequest.next(null);
  request?.resolve(result);
};
