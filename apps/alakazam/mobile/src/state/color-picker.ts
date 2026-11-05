import type { Rgb } from '@/src/lib/color/color-math';
import { BehaviorSubject } from 'rxjs';

/** A named color offered in the picker, so the common colors are one tap and not a drag. */
export type ColorSwatch = { label: string; rgb: Rgb };

/** The color a person picked, and how strong it is (255 is solid, 0 is not there). */
export type ColorPickerResult = { rgb: Rgb; alpha: number };

export type PickColorOptions = {
  /** The words over the picker. Defaults to "Pick a color". */
  title?: string;
  /** The color the picker opens on, shown as the "current" color. Defaults to white. */
  value?: Rgb;
  /** Gives the picker a strength control and says how strong the color starts, 0 to 255. Without it the color is always solid (255). */
  alpha?: number;
  /** The named colors shown above the picker. Defaults to a common set. */
  swatches?: ColorSwatch[];
  /** Called as the color changes while the person drags, for a caller that shows the change at once. `pickColor` itself answers when they press OK. */
  onChange?: (result: ColorPickerResult) => void;
};

/** An open request for the picker, which the host (`ColorPickerHost`) shows and answers. `id` is new for each request, so the host shows a new picker for it. */
export type ColorPickerRequest = { id: number; options: PickColorOptions; resolve: (result: ColorPickerResult | null) => void };

let nextRequestId = 0;

/** The request the picker is open for, or nothing when it is closed. */
export const colorPickerRequest = new BehaviorSubject<ColorPickerRequest | null>(null);
export const colorPickerRequest$ = colorPickerRequest.asObservable();

/**
 * Opens the color picker and answers with the color the person picked when they press OK, or `null` when they cancel. It is for any part of the
 * app, plugin or not. Asking while the picker is open cancels the one before.
 */
export const pickColor = (options: PickColorOptions = {}): Promise<ColorPickerResult | null> =>
  new Promise(resolve => {
    colorPickerRequest.value?.resolve(null);
    colorPickerRequest.next({ id: nextRequestId++, options, resolve });
  });

/** Closes the picker and answers the request: the color picked, or `null` for cancel. */
export const finishColorPick = (result: ColorPickerResult | null) => {
  const request = colorPickerRequest.value;
  colorPickerRequest.next(null);
  request?.resolve(result);
};
