import { clsx, type ClassValue } from 'clsx';
import { useState } from 'react';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/** Focusable element styles for the element outline. */
export const focusable = cn(
  'focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2',
  // '[&[aria-selected]]:outline-none [&[aria-selected]]:ring-2 [&[aria-selected]]:ring-primary [&[aria-selected]]:ring-offset-2',
);
/**
 * Applies functionality to a component that allows it to be selected with the Enter or Space key.
 * Returns true if the key is Enter or Space, false otherwise. It also prevents the default action of the event.
 * @param e The keyboard event to check.
 * @param preventDefault Whether to prevent the default action of the event. Defaults to true.
 */
export const keySelect = (e: React.KeyboardEvent, preventDefault = true) => {
  if (['Enter', ' '].includes(e.key)) {
    if (preventDefault) e.preventDefault();
    return true;
  }
  return false;
};

/** Clamp a number between min and max (inclusive). */
export const clamp = (n: number, min = -Infinity, max = Infinity) => Math.min(Math.max(n, min), max);

/**
 * Provides a numeric value and a string input value that can handle intermediate input states such as '-'.
 * - `num` is always a clamped, canonical number used for sliders and final application.
 * - `input` is the raw string shown in a text input; it allows '-' and empty string while typing.
 *
 * setFromInput(value): store raw input; if value parses to a number, it updates `num` and normalizes the `input` to the clamped number string.
 * setFromNumber(value): set the numeric value and the normalized string representation.
 */
export function useNumericInput(
  initial: number,
  opts?: { min?: number; max?: number; round?: boolean; step?: number },
) {
  const { min = -Infinity, max = Infinity, round = true } = opts ?? {};
  // Convert step to a number if provided, otherwise default to 1 or 0.1 based on rounding
  // Examples:
  // - step=0.1; value=4.123; result=4.1
  // - step=0.01; value=4.123; result=4.12
  // - step=1; value=4.123; result=4
  const step = opts?.step ?? (round ? 1 : 0.1);
  const decimals = Math.max(0, -Math.floor(Math.log10(step)));
  const initialRounded = Number((Math.round(initial / step) * step).toFixed(decimals));
  const initialClamped = clamp(initialRounded, min, max);
  const [num, setNum] = useState<number>(initialClamped);
  const [input, setInput] = useState<string>(initialClamped.toFixed(decimals));

  const setFromNumber = (value: number) => {
    const rounded = Math.round(value / step) * step;
    const val = clamp(Number(rounded.toFixed(decimals)), min, max);
    setNum(val);
    setInput(val.toFixed(decimals));
  };

  const setFromInput = (value: string) => {
    if (!/^-?\d*\.?\d*$/.test(value)) {
      return;
    }
    setInput(value);
    // Allow intermediate values like '-' or empty string
    // - Allow '', '-', '.', '-.' as they are obviously intermediate and don't change `num`.
    if (value === '' || value === '-' || value === '.' || value === '-.') return;
    // - If the value begins with a dot (".5" or "-.5") or ends with a dot ("1."),
    //   update the numeric `num` value but keep the raw `input` string so the user
    //   can continue typing without the input being normalized.
    if (/^-?\.\d+$/.test(value) || /^\d+\.$/.test(value)) {
      const n = Number(value);
      if (!Number.isNaN(n)) {
        const rounded = Math.round(n / step) * step;
        const val = clamp(Number(rounded.toFixed(decimals)), min, max);
        setNum(val);
      }
      return;
    }
    const n = Number(value);
    if (!Number.isNaN(n)) {
      const rounded = Math.round(n / step) * step;
      const val = clamp(Number(rounded.toFixed(decimals)), min, max);
      setNum(val);
      setInput(val.toFixed(decimals));
    }
  };

  const setter = (valueOrFn: number | string | ((prev: { num: number; input: string }) => number | string)) => {
    if (typeof valueOrFn === 'function') {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const fn = valueOrFn as any;
      const result = fn({ num, input });
      return setter(result);
    }
    if (typeof valueOrFn === 'number') return setFromNumber(valueOrFn);
    return setFromInput(valueOrFn);
  };

  return [{ num, input, min, max, round, step }, setter] as const;
}

export type NumericInputValue = ReturnType<typeof useNumericInput>[0];
