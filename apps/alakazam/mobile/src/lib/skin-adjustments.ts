import { SKIN_SMOOTH_KEY, SKIN_TAN_KEY, SKIN_TONE_KEY } from '@/src/lib/edit-sections/beauty';

export function isSkinAdjustment(key: string): boolean {
  return key === SKIN_SMOOTH_KEY || key === SKIN_TAN_KEY || key === SKIN_TONE_KEY;
}

export function adjustmentKey(key: string, personId: number | null): string {
  return isSkinAdjustment(key) && personId !== null ? `${key}:person:${personId}` : key;
}

export function skinAdjustmentTargets(
  values: Record<string, number>,
  key: string,
): { key: string; personId: number | null; value: number }[] {
  const targets: { key: string; personId: number | null; value: number }[] = [];
  if (values[key] !== undefined) targets.push({ key, personId: null, value: values[key] });
  const prefix = `${key}:person:`;
  for (const [storedKey, value] of Object.entries(values)) {
    if (!storedKey.startsWith(prefix)) continue;
    const personId = Number(storedKey.slice(prefix.length));
    if (!Number.isInteger(personId) || personId < 0) throw new Error(`Invalid skin adjustment target: ${storedKey}`);
    targets.push({ key: storedKey, personId, value });
  }
  return targets;
}
