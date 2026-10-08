import { unstable_batchedUpdates } from 'react-native';

/**
 * Runs `fn` so every subject emission inside it re-renders subscribed components once, at the end.
 * Each `useObservable` update is otherwise a synchronous render, and a burst of them from one edit
 * (replay, save, undo) can re-enter React mid-render and throw "Should not already be working".
 */
export function batch<T>(fn: () => T): T {
  let result!: T;
  unstable_batchedUpdates(() => {
    result = fn();
  }, undefined);
  return result;
}
