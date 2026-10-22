import { useEffect, useRef, useState } from 'react';

type Keep = {
  <V>(value: V): V;
  <V extends unknown[]>(...values: V): V;
};

/** A native object that the library handed back, which has to be freed. */
export type Handle = { uniffiDestroy(): void };

/** Whether `value` is a native object. This is the only place that knows how one is told apart. */
export const isHandle = (value: unknown): value is Handle =>
  typeof value === 'object' && value !== null && 'uniffiDestroy' in value;

/**
 * Frees native objects now, for the ones that outlive a function or a component (caches, the open photo): whoever replaces or drops
 * one releases it. Nothing (`null`, `undefined`) is skipped. This and `keep` are the only ways a handle is freed; nothing else
 * calls `uniffiDestroy`.
 */
export function release(...values: unknown[]) {
  for (const value of values) {
    if (value == null) continue;
    // A subscription that lives and ends with a handle can be kept beside it, so the two never get out of step.
    if (typeof (value as { unsubscribe?: unknown }).unsubscribe === 'function') (value as { unsubscribe(): void }).unsubscribe();
    else (value as Handle).uniffiDestroy();
  }
}

/** What `keep` has been given so far, and the way to free all of it, last kept first. Nothing (`null`, `undefined`) is skipped. */
function scope() {
  const owned: unknown[] = [];
  const free = () => release(...owned.reverse());
  const keep = ((...values: unknown[]) => {
    owned.push(...values.filter((value) => value != null));
    return values.length === 1 ? values[0] : values;
  }) as Keep;
  return { keep, free };
}

/**
 * Runs `work`, then frees everything it kept, as the editor does for its images. Whatever the library hands back is kept, either
 * one value at a time or several in one call. When `work` is async, what it kept is freed once it is done.
 */
export function owning<T>(work: (keep: Keep) => T): T {
  const { keep, free } = scope();
  let result: T;
  try {
    result = work(keep);
  } catch (error) {
    free();
    throw error;
  }
  if (result instanceof Promise) return result.finally(free) as T;
  free();
  return result;
}

/**
 * The same as `owning`, for a component: what `work` keeps lives as long as the component uses it, and is freed when `deps` change
 * or the component goes away. It is made in an effect, so React re-running the effect never leaves a freed handle behind (the
 * "Raw pointer value was null" error). It is `null` for the first render, so the caller waits for it. Never keep a handle in
 * `useMemo` or `useRef` and free it from an effect. State is a render behind, so code that runs between renders (a gesture, a
 * subscription) reads the handle through `alive`, a ref this sets when the handle is made and clears when it is replaced. An effect
 * that uses the handle reads it through `alive` too: Fast Refresh re-runs effects with the value they were rendered with, which
 * may be the one that was just replaced.
 *
 * A replaced handle is freed only after the render that stopped using it has committed. Until then a component that renders on its
 * own (a native view with a state of its own) still holds the old handle in its props, and freeing it sooner is the same error.
 */
export function useOwning<T>(work: (keep: Keep) => T, deps: readonly unknown[], alive?: { current: T | null }): T | null {
  const [value, setValue] = useState<T | null>(null);
  const retired = useRef<(() => void)[]>([]);
  const flush = () => retired.current.splice(0).forEach((free) => free());
  useEffect(() => {
    const { keep, free } = scope();
    const made = work(keep);
    if (alive) alive.current = made;
    setValue(made);
    return () => {
      if (alive) alive.current = null;
      setValue(null);
      retired.current.push(free);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);
  // Runs once the new value is on screen.
  useEffect(flush, [value]);
  // And when the component goes away, nothing renders after that. Fast Refresh and strict mode run this cleanup too and then mount
  // again straight away, so it waits a tick and frees only if the component did not come back (the new value frees the rest).
  const mounted = useRef(false);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      setTimeout(() => {
        if (!mounted.current) flush();
      }, 0);
    };
  }, []);
  return value;
}
