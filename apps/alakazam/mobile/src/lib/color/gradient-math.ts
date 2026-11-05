import type { Rgb } from '@/src/lib/color/color-math';
import type { GradientValue } from '@/src/state/gradient-picker';

export const byPlace = <T extends { at: number }>(stops: T[]) => [...stops].sort((a, b) => a.at - b.at);

/** A value between the stops around `at`, mixed in proportion to how near each is, and the first or last stop's value outside them. */
const mixed = <T extends { at: number }>(stops: T[], at: number, read: (stop: T) => number[]): number[] => {
  const sorted = byPlace(stops);
  const next = sorted.findIndex(stop => stop.at >= at);
  if (next === -1) return read(sorted[sorted.length - 1]);
  if (next === 0) return read(sorted[0]);
  const [before, after] = [sorted[next - 1], sorted[next]];
  const span = after.at - before.at;
  const along = span > 0 ? (at - before.at) / span : 0;
  const to = read(after);
  return read(before).map((value, index) => value + (to[index] - value) * along);
};

/** The color of the gradient at `at`, from 0 to 1. */
export const colorAt = (value: GradientValue, at: number) => mixed(value.colors, at, stop => stop.color).map(Math.round) as Rgb;

/** How strong the gradient is at `at`, from 0 to 255. */
export const alphaAt = (value: GradientValue, at: number) => Math.round(mixed(value.alphas, at, stop => [stop.alpha])[0]);

/** The gradient as one list of stops with a color and a strength each, at every place either list has a stop, which is what draws it. */
export const stopsOf = (value: GradientValue) =>
  [...new Set([...value.colors, ...value.alphas].map(stop => stop.at))]
    .sort((a, b) => a - b)
    .map(at => ({ at, rgb: colorAt(value, at), alpha: alphaAt(value, at) }));

/** The gradient with each list in order along it. */
export const sortedValue = (value: GradientValue): GradientValue => ({ colors: byPlace(value.colors), alphas: byPlace(value.alphas) });
