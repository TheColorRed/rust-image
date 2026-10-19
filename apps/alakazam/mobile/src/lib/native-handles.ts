/**
 * Runs `work`, then frees everything it kept, as the editor does for its images. Whatever the library hands back is kept. When
 * `work` is async, what it kept is freed once it is done.
 */
export function owning<T>(work: (keep: <V>(value: V) => V) => T): T {
  const owned: unknown[] = [];
  const free = () => owned.forEach((value) => (value as { uniffiDestroy(): void }).uniffiDestroy());
  let result: T;
  try {
    result = work((value) => (owned.push(value), value));
  } catch (error) {
    free();
    throw error;
  }
  if (result instanceof Promise) return result.finally(free) as T;
  free();
  return result;
}
