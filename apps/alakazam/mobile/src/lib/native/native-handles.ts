type Keep = {
  <V>(value: V): V;
  <V extends unknown[]>(...values: V): V;
};

/**
 * Runs `work`, then frees everything it kept, as the editor does for its images. Whatever the library hands back is kept, either
 * one value at a time or several in one call. When `work` is async, what it kept is freed once it is done.
 */
export function owning<T>(work: (keep: Keep) => T): T {
  const owned: unknown[] = [];
  const free = () => owned.forEach((value) => (value as { uniffiDestroy(): void }).uniffiDestroy());
  const keep = ((...values: unknown[]) => {
    owned.push(...values);
    return values.length === 1 ? values[0] : values;
  }) as Keep;
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
