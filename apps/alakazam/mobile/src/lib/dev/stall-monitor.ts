import { devDebug } from '@/src/lib/log';
import { aiProgress, aiProgress$ } from '@/src/state/ai-progress';

/** How often the monitor asks the JavaScript thread to run a timer, in milliseconds. */
const TICK_MS = 20;

/**
 * For development: logs every stretch of at least `minimumMs` in which the JavaScript thread could not run, which is when the screen does not
 * answer touches, with the AI tasks that were running then. A timer that is due every `TICK_MS` and runs late shows how long the thread was
 * held by something else (a synchronous native call, a model loading). It does nothing in a release build. Call the result to stop it.
 */
export function watchForStalls(label: string, minimumMs = 100): () => void {
  if (!__DEV__) return () => {};
  let last = Date.now();
  const timer = setInterval(() => {
    const now = Date.now();
    const stalled = now - last - TICK_MS;
    last = now;
    if (stalled >= minimumMs) {
      devDebug('stall', label, `${stalled} ms; AI tasks:`, Object.keys(aiProgress.value).join(', ') || 'none');
    }
  }, TICK_MS);
  return () => clearInterval(timer);
}

/**
 * For development: watches for stalls for as long as any AI task is running, whatever started it (a photo opening, a control, the button that
 * finds the people again). Call it once when the app starts. It does nothing in a release build.
 */
export function watchAiWorkForStalls(): void {
  if (!__DEV__) return;
  let stop: (() => void) | undefined;
  aiProgress$.subscribe(tasks => {
    const running = Object.keys(tasks).length > 0;
    if (running && !stop) stop = watchForStalls('while the AI works');
    else if (!running && stop) {
      stop();
      stop = undefined;
    }
  });
}
