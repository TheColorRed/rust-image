import { release } from '@/src/lib/native/native-handles';
import type { CurveValue } from '@/src/state/curve-picker';
import { Curve } from '@abra/core/geometry';

/**
 * The value of a curve at `count + 1` evenly spaced places from its start to its end, worked out by the library's own curve, so what is drawn is
 * the line the plugin's curve really makes. A curve is smooth through its points, and is flat before the first and after the last.
 */
export const curveSamples = (points: CurveValue, count: number): number[] => {
  const curve = new Curve([...points].sort((a, b) => a[0] - b[0]).map(([x, y]) => ({ x: x * 255, y })));
  const samples = Array.from({ length: count + 1 }, (_, at) => curve.value((at / count) * 255));
  release(curve);
  return samples;
};

/** The curve with each point in order along it. */
export const sortedCurve = (points: CurveValue): CurveValue => [...points].sort((a, b) => a[0] - b[0]);
