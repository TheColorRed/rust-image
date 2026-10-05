import type { ImagePreview } from '@alakazam/mobile';

const FRAME_TIMEOUT_MS = 5000;

/** Waits for native presentation, not a guessed delay. Cancelling prevents a retired preview from completing a handoff. */
export function waitForPreviewFrame(
  preview: Pick<ImagePreview, 'hasFrame'>,
  onReady: () => void,
  onError: (error: Error) => void,
): () => void {
  const deadline = Date.now() + FRAME_TIMEOUT_MS;
  let cancelled = false;
  let frame: number;
  const check = () => {
    if (cancelled) return;
    let ready: boolean;
    try {
      ready = preview.hasFrame();
    } catch (error) {
      onError(error instanceof Error ? error : new Error(String(error)));
      return;
    }
    if (ready) {
      onReady();
    } else if (Date.now() >= deadline) {
      onError(new Error('The edited preview did not reach the display.'));
    } else {
      frame = requestAnimationFrame(check);
    }
  };
  frame = requestAnimationFrame(check);
  return () => {
    cancelled = true;
    cancelAnimationFrame(frame);
  };
}
