// Development only: a ledger of the native handles that are alive and where each was made, so a handle that nobody released shows up the
// first time the code runs, and not as memory that is held until the garbage collector gets to it.
//
// Every class the bindings generate registers each handle with a `FinalizationRegistry`, which frees it if its wrapper is garbage-collected
// without being released, and cancels that registration when `uniffiDestroy()` runs (what `release` does). So the handles that are
// registered and not cancelled are the handles that are alive. This wraps that registry to keep them, with the stack that made each. It
// never frees anything. It has to be installed before the bindings load (index.js imports this file first), and it does nothing in a
// release build.

type Entry = { sequence: number; made: Error; held: unknown };
type Registry = {
  register(object: object, held: unknown, token?: object): void;
  unregister(token: object): boolean | void;
};

/** The handles that are alive. One entry for each registration: two wrappers can hold the same pointer (Rust can hand out one object twice). */
const alive = new Set<Entry>();
/** The entries of each pointer, oldest first, for the registry's own callback, which is only given the pointer. */
const byHeld = new Map<unknown, Entry[]>();
/** The entry of each registration, found by the token (or object) it was registered with. */
const byToken = new WeakMap<object, Entry>();
let sequence = 0;

const forget = (entry: Entry) => {
  alive.delete(entry);
  const entries = byHeld.get(entry.held);
  if (!entries) return;
  const index = entries.indexOf(entry);
  if (index >= 0) entries.splice(index, 1);
  if (entries.length === 0) byHeld.delete(entry.held);
};

/** The frames of the generated code and of this file, which say nothing about who made the handle. */
const generated = /^(create|lift|lower|bless|unbless|clonePointer|rustCall|register|handleLedger|FfiConverter\w*|uniffi\w*)$/;

/** The names of the functions that made a handle, innermost first, without the generated ones. */
const whereOf = (made: Error): string => {
  const names = (made.stack ?? '')
    .split('\n')
    .map(line => line.match(/at (?:new )?(\S+) \(/)?.[1])
    .filter((name): name is string => !!name && !generated.test(name));
  return names.slice(0, 5).join(' < ') || 'unknown';
};

/** Makes `target.FinalizationRegistry` a registry that also keeps the ledger. Does nothing when it has none or already does. */
export function installHandleLedger(target: { FinalizationRegistry?: unknown }) {
  const Original = target.FinalizationRegistry as (new (cleanup: (held: unknown) => void) => Registry) & { keepsLedger?: boolean };
  if (!Original || Original.keepsLedger) return;
  const Ledger = function (cleanup: (held: unknown) => void) {
    // The registry's own callback runs when a handle was collected unreleased: it is freed (as before) and is no longer alive.
    const inner = new Original(held => {
      const entry = byHeld.get(held)?.[0];
      if (entry) forget(entry);
      cleanup(held);
    });
    return {
      register(object: object, held: unknown, token?: object) {
        const entry = { sequence: ++sequence, made: new Error('where a native handle was made'), held };
        alive.add(entry);
        const entries = byHeld.get(held);
        if (entries) entries.push(entry);
        else byHeld.set(held, [entry]);
        byToken.set(token ?? object, entry);
        inner.register(object, held, token);
      },
      unregister(token: object) {
        const entry = byToken.get(token);
        if (entry) forget(entry);
        return inner.unregister(token);
      },
    };
  } as unknown as { keepsLedger?: boolean };
  Ledger.keepsLedger = true;
  target.FinalizationRegistry = Ledger;
}

/** A point in time: the handles made after it are the ones `unreleasedSince` reports. */
export const handleLedgerMark = (): number => sequence;

/** The handles made after `mark` that are still alive, grouped by the functions that made them, the most common first. */
export function unreleasedSince(mark: number): { where: string; count: number; first: Error }[] {
  const groups = new Map<string, { where: string; count: number; first: Error }>();
  for (const { sequence: made, made: error } of alive) {
    if (made <= mark) continue;
    const where = whereOf(error);
    const group = groups.get(where);
    if (group) group.count++;
    else groups.set(where, { where, count: 1, first: error });
  }
  return [...groups.values()].sort((a, b) => b.count - a.count);
}

/** Warns, once per place that made them, about the handles made after `mark` that are still alive. `label` says what ended. */
export function warnUnreleasedSince(mark: number, label: string) {
  const groups = unreleasedSince(mark);
  if (groups.length === 0) return;
  const total = groups.reduce((sum, group) => sum + group.count, 0);
  console.warn(`[leak] ${total} native handles made before ${label} are still alive, from ${groups.length} places:`);
  for (const { where, count, first } of groups) console.warn(`[leak] ${count} x ${where}`, first);
}

if (__DEV__) installHandleLedger(globalThis as { FinalizationRegistry?: unknown });
