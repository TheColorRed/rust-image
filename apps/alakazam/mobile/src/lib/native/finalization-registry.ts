// A `FinalizationRegistry` for engines that have `WeakRef` and not `FinalizationRegistry`, which is the case of the Hermes this app runs on.
//
// Every class the bindings generate makes one registry (`typeof FinalizationRegistry !== 'undefined' ? new FinalizationRegistry(free) : null`)
// and registers each native handle with it, so that a handle nobody released is freed when its wrapper is garbage-collected. Without a
// registry that net does not exist: a handle that is not released is never freed. This one finds the collected wrappers by asking their
// `WeakRef`s, a little after each registration and then every few seconds while any handle is registered. It does not make the garbage
// collector run: a wrapper is only noticed once Hermes has collected it, so releasing a handle is still what frees it at once.
//
// It has to be installed before the bindings load (index.js imports this file first).

type Entry = { reference: WeakRef<object>; held: unknown };
type Engine = { FinalizationRegistry?: unknown; WeakRef?: unknown };

/** How often the registered wrappers are checked while there are any, in milliseconds. */
export const SWEEP_MS = 2000;

/** Installs the registry on `target` when it has none and has `WeakRef`. Returns what a test needs to run a check by hand. */
export function installFinalizationRegistry(target: Engine, sweepMs = SWEEP_MS) {
  if (target.FinalizationRegistry || typeof target.WeakRef !== 'function') return undefined;
  const Reference = target.WeakRef as new (object: object) => WeakRef<object>;
  const registries = new Set<PollingRegistry>();
  let timer: ReturnType<typeof setTimeout> | undefined;

  const sweep = () => {
    timer = undefined;
    let waiting = false;
    for (const registry of registries) {
      registry.collect();
      if (registry.size() > 0) waiting = true;
      else registries.delete(registry);
    }
    if (waiting) timer = setTimeout(sweep, sweepMs);
  };

  class PollingRegistry {
    private readonly entries = new Set<Entry>();
    /** The entry of each token, so `unregister` does not look through every handle. */
    private readonly byToken = new WeakMap<object, Entry>();

    constructor(private readonly cleanup: (held: unknown) => void) {}

    register(object: object, held: unknown, token?: object) {
      const entry = { reference: new Reference(object), held };
      this.entries.add(entry);
      if (token) this.byToken.set(token, entry);
      registries.add(this);
      timer ??= setTimeout(sweep, sweepMs);
    }

    unregister(token: object) {
      const entry = this.byToken.get(token);
      if (!entry) return false;
      this.byToken.delete(token);
      return this.entries.delete(entry);
    }

    size() {
      return this.entries.size;
    }

    /** Frees the handles whose wrappers have been collected. A failure of one does not stop the rest. */
    collect() {
      for (const entry of [...this.entries]) {
        if (entry.reference.deref() !== undefined) continue;
        this.entries.delete(entry);
        try {
          this.cleanup(entry.held);
        } catch (error) {
          console.warn('[native] could not free a handle whose wrapper was collected:', error);
        }
      }
    }
  }

  target.FinalizationRegistry = PollingRegistry;
  return { sweep };
}

installFinalizationRegistry(globalThis as Engine);
