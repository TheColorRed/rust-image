import { useEffect, useMemo } from 'react';
import { useObservable } from 'react-rx';
import { type SliderControl } from '@/src/lib/edit-sections';
import { ensureBase, editorLive } from '@/src/lib/editor-live';
import { hasOtherSteps, renderStackWithoutSlider } from '@/src/lib/editor-replay';
import { adjustmentKey } from '@/src/lib/skin-adjustments';
import { personSelection$, personSelection } from '@/src/state/person-selection';
import { appliedActions$, adjustments$ } from '@/src/state/edits';
import { previewBox$ } from '@/src/state/gestures';

/**
 * Live GPU preview for a slider drag. Returns the change handler to give the slider, or undefined when the control has no
 * live form. Frames are drawn over the editor preview until the released value is committed and replayed.
 *
 * The drag is shown by the editor's live renderer, which is already connected to the GPU: it only tells Rust which slider
 * moved and to what value. With nothing else applied the effect goes over the untouched edit base. Otherwise the rest of
 * the stack is rendered on the CPU once, and only the dragged slider's effect runs on top of it.
 */
export function useLiveSlider(p_control: SliderControl | undefined): ((value: number) => void) | undefined {
  const box = useObservable(previewBox$, null);
  const adjustments = useObservable(adjustments$, {});
  const actions = useObservable(appliedActions$, []);
  const selection = useObservable(personSelection$, personSelection.value);
  const live = p_control?.live;
  const key = p_control?.key;

  // Everything except this slider changes what a drag should start from; its own value does not.
  const targetKey = adjustmentKey(key ?? '', selection.selectedId);
  const others = useMemo(
    () => JSON.stringify([key, selection.selectedId, actions, { ...adjustments, [targetKey]: 0 }]),
    [key, selection.selectedId, targetKey, actions, adjustments],
  );

  // Open the preview this drag needs before it starts: the shared edit base, or the CPU-rendered rest of the stack.
  useEffect(() => {
    if (!live || !key || !box) return;
    if (!hasOtherSteps(key)) {
      ensureBase();
      return;
    }
    editorLive.ensure(`rest:${others}`, () => renderStackWithoutSlider(key), true);
  }, [live, key, box, others]);

  // When the slider closes its frames go away, and the renderer goes back to the edit base for the next tap or drag.
  useEffect(
    () => () => {
      editorLive.hide();
      ensureBase();
    },
    [],
  );

  if (!live || !key || !box) return undefined;
  return value => editorLive.slide(key, value);
}
