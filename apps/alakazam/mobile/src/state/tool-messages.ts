import { BehaviorSubject } from 'rxjs';

/** Short, temporary feedback for the currently open tool, keyed by its own control id. */
export const toolMessages = new BehaviorSubject<Record<string, string>>({});
export const toolMessages$ = toolMessages.asObservable();

const timers = new Map<string, ReturnType<typeof setTimeout>>();

export function setToolMessage(key: string, message: string | null): void {
  const timer = timers.get(key);
  if (timer) clearTimeout(timer);
  timers.delete(key);
  const next = { ...toolMessages.value };
  if (message) {
    next[key] = message;
    timers.set(key, setTimeout(() => setToolMessage(key, null), 4000));
  } else delete next[key];
  toolMessages.next(next);
}
