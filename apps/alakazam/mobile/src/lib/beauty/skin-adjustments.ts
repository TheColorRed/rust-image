import type { AdjustmentValue } from '@/src/lib/project/edit-sections';
import { isPersonControlKey } from '@/src/state/control-traits';

/** The controls that can be set for one person, or for everyone. */
export function isPersonAdjustment(key: string): boolean {
  return isPersonControlKey(key);
}

export function adjustmentKey(key: string, personId: number | null): string {
  return isPersonAdjustment(key) && personId !== null ? `${key}:person:${personId}` : key;
}

/** The stored value is one number for a single slider, or an array for a multi-slider makeup control. */
export function personAdjustmentTargets(
  values: Record<string, AdjustmentValue>,
  key: string,
): { key: string; personId: number | null; value: AdjustmentValue }[] {
  const targets: { key: string; personId: number | null; value: AdjustmentValue }[] = [];
  if (values[key] !== undefined) targets.push({ key, personId: null, value: values[key] });
  const prefix = `${key}:person:`;
  for (const [storedKey, value] of Object.entries(values)) {
    if (!storedKey.startsWith(prefix)) continue;
    const personId = Number(storedKey.slice(prefix.length));
    if (!Number.isInteger(personId) || personId < 0) throw new Error(`Invalid person adjustment target: ${storedKey}`);
    targets.push({ key: storedKey, personId, value });
  }
  return targets;
}

/** The box of a detected person, shrunk to a photo that is `scale` times the size the person was found in. */
export const scaledPersonBounds = <P extends { x: number; y: number; width: number; height: number }>(
  person: P | undefined,
  scale: number,
) =>
  person && {
    ...person,
    x: person.x * scale,
    y: person.y * scale,
    width: person.width * scale,
    height: person.height * scale,
  };
