import type { AdjustmentValue } from '@/src/lib/edit-sections';
import { SKIN_SMOOTH_KEY, SKIN_TAN_KEY, SKIN_TONE_KEY } from '@/src/lib/edit-sections/keys';

export function isSkinAdjustment(key: string): boolean {
  return key === SKIN_SMOOTH_KEY || key === SKIN_TAN_KEY || key === SKIN_TONE_KEY;
}

export function adjustmentKey(key: string, personId: number | null): string {
  return isSkinAdjustment(key) && personId !== null ? `${key}:person:${personId}` : key;
}

/** The skin controls each have one slider, so their stored values are numbers. */
export function skinAdjustmentTargets(
  values: Record<string, AdjustmentValue>,
  key: string,
): { key: string; personId: number | null; value: number }[] {
  const targets: { key: string; personId: number | null; value: number }[] = [];
  if (values[key] !== undefined) targets.push({ key, personId: null, value: values[key] as number });
  const prefix = `${key}:person:`;
  for (const [storedKey, value] of Object.entries(values)) {
    if (!storedKey.startsWith(prefix)) continue;
    const personId = Number(storedKey.slice(prefix.length));
    if (!Number.isInteger(personId) || personId < 0) throw new Error(`Invalid skin adjustment target: ${storedKey}`);
    targets.push({ key: storedKey, personId, value: value as number });
  }
  return targets;
}
