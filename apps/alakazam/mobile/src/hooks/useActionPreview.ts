import { useEffect } from 'react';
import { effectList } from '@/src/lib/edit-sections';
import { ensureBase, editorPreview, showTool } from '@/src/lib/editor-preview';
import { restForAction } from '@/src/lib/editor-replay';
import { actionPreviewRequested$ } from '@/src/state/commands';

let restCount = 0;

/**
 * Instant preview for tapping an action that has a live form or a tool. The tap tells the editor's live renderer, which is
 * already connected to the GPU, the action's effects, so only the effects cost anything; a tool is run on a screen-sized
 * copy of the photo and its result shown. The full-resolution replay that follows replaces the frames once it is done, as
 * after a slider release.
 *
 * The `live` path (`ActionControl.live`) is deprecated: it only serves controls that have not moved to `tool` yet, and can be
 * removed with it, leaving only the tool path.
 */
export function useActionPreview() {
  useEffect(() => {
    const subscription = actionPreviewRequested$.subscribe(control => {
      const tool = control.tool;
      if (!control.live && !tool) return;
      const rest = restForAction(control);
      // The CPU-rendered rest is different every time, so it always opens a new preview.
      const ready = rest ? editorPreview.ensure(`action:${++restCount}`, rest, true) : ensureBase();
      if (!ready) return;
      if (control.live) editorPreview.showEffects(effectList(control.live()));
      else if (tool) showTool(tool);
    });
    return () => subscription.unsubscribe();
  }, []);
}
