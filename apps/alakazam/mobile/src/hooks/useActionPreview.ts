import { useEffect } from 'react';
import { ensureBase, editorPreview, showTool } from '@/src/lib/editor/editor-preview';
import { isStackDeferred, restForAction } from '@/src/lib/editor/editor-replay';
import { actionPreviewRequested$ } from '@/src/state/commands';
import { editBaseImage } from '@/src/state/session';

let restCount = 0;

/**
 * Instant preview for tapping an action that has a tool. The tool is run on a screen-sized copy of the photo and its result
 * shown. The full-resolution replay that follows replaces the frames once it is done, as after a slider release. When the
 * action and everything applied with it only make the photo remember effects for the GPU, the tool is run on the full-size
 * photo instead, which costs the same, and what is shown is sharp when zoomed in.
 */
export function useActionPreview() {
  useEffect(() => {
    const subscription = actionPreviewRequested$.subscribe(control => {
      const tool = control.tool;
      if (!tool) return;
      const base = editBaseImage.value;
      if (control.deferred && base && isStackDeferred(undefined, control)) {
        if (!ensureBase()) return;
        const rest = restForAction(control, true)?.();
        try {
          showTool(tool, rest ?? base);
        } finally {
          rest?.uniffiDestroy();
        }
        return;
      }
      const rest = restForAction(control);
      // A repeatable action builds on what is already applied, so its tool runs on the rest of the stack.
      const start = control.repeatable && rest ? rest() : undefined;
      try {
        // The CPU-rendered rest is different every time, so it always opens a new preview.
        const ready = rest ? editorPreview.ensure(`action:${++restCount}`, () => start ?? rest(), !start) : ensureBase();
        if (!ready) return;
        showTool(tool, start);
      } finally {
        start?.uniffiDestroy();
      }
    });
    return () => subscription.unsubscribe();
  }, []);
}
