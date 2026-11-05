/**
 * Numbers that cross between the editor and a plugin are held to being real numbers: not NaN, not infinite. A bad number is easy to make (a
 * missing field, a value read back from a save, a division by zero) and hard to find, because it goes on to the library as a radius or a size
 * and the picture is just blank. The editor does this for every plugin, so no plugin has to remember to: it fixes what a plugin gives it, and it
 * does not give a plugin a number that is not real.
 */

/** Whether the value is a real number: a number that is not NaN and not infinite. */
export const isReal = (value: unknown): value is number => typeof value === 'number' && Number.isFinite(value);

/** The value when it is a real number, and `fallback` when it is not. */
export const realOr = (value: unknown, fallback: number): number => (isReal(value) ? value : fallback);

/** The value held between `low` and `high`, or `fallback` (held in the same way) when it is not a real number. */
export const realBetween = (value: unknown, low: number, high: number, fallback: number): number =>
  Math.min(high, Math.max(low, realOr(value, fallback)));

/** The plugins that have already been told about a number that was not real, so a plugin that gives one every time its inspector is made is told once. */
const told = new Set<string>();

/**
 * Says in the log, once for each plugin and thing, that the plugin gave the editor a number that was not real and what the editor used instead.
 * This is for the person writing the plugin: the picture still works, and the log says where the bad number came from.
 */
export const warnNotReal = (pluginId: string, what: string, used: unknown) => {
  const key = `${pluginId}:${what}`;
  if (told.has(key)) return;
  told.add(key);
  console.warn(`The plugin ${pluginId} gave ${what} a number that is not real (NaN or infinite), so the editor used ${String(used)} instead.`);
};

/** Whether a value holds a number that is not real anywhere inside it, at any depth. */
export const holdsNotReal = (value: unknown, depth = 0): boolean => {
  if (typeof value === 'number') return !Number.isFinite(value);
  if (depth > 8 || typeof value !== 'object' || value === null) return false;
  return Object.values(value).some(each => holdsNotReal(each, depth + 1));
};
