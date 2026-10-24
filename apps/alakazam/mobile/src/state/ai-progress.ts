import { devDebug } from '@/src/lib/log';
import { BehaviorSubject } from 'rxjs';
import { debounceTime, filter } from 'rxjs/operators';

/**
 * The AI work of one job, by what it is (`skin`, `people`, `faces`). A value from 0 to 1 is how far along it is, 1 is finished, and
 * null is a model that does not say. A task sets null (or its progress) when it starts and 1 when it ends. A task that fails takes
 * itself out instead, because it did no work to count (a model that is not on the device fails at once). A task stays in the list when it
 * ends, so the bar counts it until the whole job is done and then clears it. No tasks means the bar is hidden.
 */
export const aiProgress = new BehaviorSubject<Record<string, number | null>>({});
export const aiProgress$ = aiProgress.asObservable();

/** How long a finished job shows its full bar before it clears, so a short one is seen and the next job starts from nothing. */
const DONE_MS = 500;

// Clears the job once every task is finished and nothing has started in the meantime (the wait comes first, so the latest tasks are read).
aiProgress$
  .pipe(
    debounceTime(DONE_MS),
    filter(tasks => Object.keys(tasks).length > 0 && Object.values(tasks).every(progress => progress === 1)),
  )
  .subscribe(() => aiProgress.next({}));

aiProgress$.subscribe(tasks => devDebug('ai', 'tasks:', JSON.stringify(tasks)));

/**
 * How far along the whole job is, from 0 to 1. Every task is worth the same share of it, because how long each takes is not known
 * beforehand: a finished task counts 1, one that reports counts what it reports, and one that does not report counts `silent`, the
 * caller's guess of how far it has got (0 when it has none). The share is the sum over the number of tasks. `unsure` is true while a task
 * that does not report is running, so the share is partly a guess.
 */
export const overallProgress = (tasks: Record<string, number | null>, silent = 0) => {
  const values = Object.values(tasks);
  return {
    running: values.length > 0,
    share: values.length === 0 ? 0 : values.reduce((sum: number, progress) => sum + (progress ?? silent), 0) / values.length,
    unsure: values.some(progress => progress === null),
  };
};
