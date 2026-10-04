import { useEffect } from 'react';
import { ensureBase, editorLive } from '@/src/lib/editor-live';
import { restForAction } from '@/src/lib/editor-replay';
import { actionPreviewRequested$ } from '@/src/state/commands';

let restCount = 0;

/**
 * Instant preview for tapping an action that has a live form. The tap tells the editor's live renderer, which is already
 * connected to the GPU, which action it was, so only the effects cost anything. The full-resolution replay that
 * follows replaces the frames once it is done, as after a slider release.
 */
export function useLiveAction() {
  useEffect(() => {
    const subscription = actionPreviewRequested$.subscribe(control => {
      if (!control.live) return;
      const rest = restForAction(control);
      // The CPU-rendered rest is different every time, so it always opens a new preview.
      const ready = rest ? editorLive.ensure(`action:${++restCount}`, rest, true) : ensureBase();
      if (ready) editorLive.act(control.key);
    });
    return () => subscription.unsubscribe();
  }, []);
}
