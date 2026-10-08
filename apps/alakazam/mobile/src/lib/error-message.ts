/** UniFFI record errors carry the native explanation in `inner`, not Error.message. */
export function errorMessage(error: unknown): string {
  if (typeof error === 'object' && error !== null) {
    if ('inner' in error && typeof error.inner === 'object' && error.inner !== null) {
      if ('message' in error.inner && typeof error.inner.message === 'string') return error.inner.message;
    }
    if ('message' in error && typeof error.message === 'string') return error.message;
  }
  return String(error);
}
