import { useEffect, useMemo, useRef } from 'react';
import { AbraImage } from '@alakazam/mobile';
import { useObservable } from 'react-rx';
import { filter, take } from 'rxjs/operators';
import { areaPoints, layerSheet, skinMaskFor, type SliderControl } from '@/src/lib/edit-sections';
import { preloadFor } from '@/src/lib/editor/editor-preloads';
import { ensureBase, editorPreview, showTool } from '@/src/lib/editor/editor-preview';
import { hasOtherSteps, isStackDeferred, renderStackWithoutSlider, renderStepsAfterSlider } from '@/src/lib/editor/editor-replay';
import { withLayers } from '@/src/lib/layers';
import { adjustmentKey, personAdjustmentTargets, scaledPersonBounds } from '@/src/lib/skin-adjustments';
import { personSelection$, personSelection } from '@/src/state/person-selection';
import { appliedActions$, adjustments$ } from '@/src/state/edits';
import { editBaseImage } from '@/src/state/session';
import { previewBox$, previewMoving$ } from '@/src/state/gestures';

/**
 * Preview for a slider drag. Returns the change handler to give the slider, or undefined when the control has no tool or
 * layer. Frames are drawn over the editor preview until the released value is committed and replayed.
 *
 * The handler takes the value of every slider of the control (one for a control with one slider) each time any one moves.
 * With nothing else applied the tool runs over the untouched edit base. Otherwise the rest of the stack is rendered on the
 * CPU once, and only the dragged slider's tool runs on top of it.
 */
export function useSliderPreview(p_control: SliderControl | undefined): ((values: number[]) => void) | undefined {
  const box = useObservable(previewBox$, null);
  const adjustments = useObservable(adjustments$, {});
  const actions = useObservable(appliedActions$, []);
  const selection = useObservable(personSelection$, personSelection.value);
  const tool = p_control?.tool;
  const layer = p_control?.layer;
  const key = p_control?.key;

  // Everything except this slider changes what a drag should start from; its own value does not.
  const targetKey = adjustmentKey(key ?? '', selection.selectedId);
  const others = useMemo(
    () => JSON.stringify([key, selection.selectedId, actions, { ...adjustments, [targetKey]: 0 }]),
    [key, selection.selectedId, targetKey, actions, adjustments],
  );

  // Open the preview this drag needs before it starts: the shared edit base, or the CPU-rendered rest of the stack.
  useEffect(() => {
    if (!(tool || layer) || !key || !box) return;
    // Nothing else to start from, or only controls that make the photo remember effects: the drag is made over the full-size photo.
    if (!hasOtherSteps(key) || (p_control?.deferred && isStackDeferred(key))) {
      ensureBase();
      return;
    }
    // Rendering the rest of the stack takes the JavaScript thread for half a second or more, so it waits for a move of the preview
    // (the zoom to another person) to land instead of freezing it.
    const waiting = previewMoving$
      .pipe(
        filter(moving => !moving),
        take(1),
      )
      .subscribe(() => editorPreview.ensure(`rest:${others}`, () => renderStackWithoutSlider(key), true));
    return () => waiting.unsubscribe();
  }, [tool, layer, key, box, others]);

  // When the slider closes its frames go away, and the renderer goes back to the edit base for the next tap or drag.
  useEffect(
    () => () => {
      editorPreview.hide();
      ensureBase();
    },
    [],
  );

  // A drag sends many more moves than the screen can show, and a tool runs on the JavaScript thread, so running it for each one
  // makes the picture fall behind the finger. The newest values wait here for the next frame, and moves that arrive meanwhile
  // only replace them.
  const waiting = useRef<{ values: number[]; frame: number | null }>({ values: [], frame: null });
  // The rest of the stack at screen size, which every frame of the drag starts from. It is made on the first frame after the
  // other edits change, and let go of when they change again or the slider closes.
  const rest = useRef<{ key: string; image: AbraImage | null }>({ key: '', image: null });
  useEffect(
    () => () => {
      if (waiting.current.frame !== null) cancelAnimationFrame(waiting.current.frame);
      rest.current.image?.uniffiDestroy();
      rest.current = { key: '', image: null };
    },
    [],
  );

  if (!key || !box) return undefined;
  if (tool || layer) {
    // The tool runs on the photo at the size the preview is drawn at, so a drag costs what the screen shows.
    return values => {
      const pending = waiting.current;
      pending.values = values;
      if (pending.frame !== null) return;
      pending.frame = requestAnimationFrame(() => {
        pending.frame = null;
        // A control that makes the photo remember effects is dragged on the full-size photo, so what is drawn is sharp when zoomed in.
        const full = !!p_control.deferred && isStackDeferred(key);
        if (rest.current.key !== others) {
          rest.current.image?.uniffiDestroy();
          rest.current = { key: others, image: hasOtherSteps(key) ? renderStackWithoutSlider(key, true, full) : null };
        }
        showTool((photo, context) => {
          const person = scaledPersonBounds(
            personSelection.value.people.find(found => found.id === personSelection.value.selectedId),
            context.scale,
          );
          // For everyone: leave out the people who have their own value, as the final render does.
          const ownPeople = person
            ? undefined
            : personAdjustmentTargets(adjustments, key ?? '').flatMap(target => (target.personId === null ? [] : [target.personId]));
          const toolContext = {
           ...context,
           preloaded: preloadFor(p_control, photo.width()),
           person,
           others: ownPeople,
           skin: () => skinMaskFor(person?.id, photo, ownPeople),
           // The drawn areas the drag is kept in, as the committed replay gets them.
           areas: p_control.areaKeys?.map(areaKey => areaPoints(adjustments[areaKey])),
          };
          // A control with a layer paints its own sheet, which is shown over the photo; the photo itself is not changed.
          if (p_control.layer) {
            const laid = withLayers(photo, [layerSheet(p_control, pending.values, toolContext, targetKey)]) as AbraImage;
            renderStepsAfterSlider(key, laid);
            return laid;
          }
          // The steps that come after this control in the stack (makeup over the skin) go back over what the drag made.
          const made = (tool!(photo, pending.values, toolContext) ?? photo) as AbraImage;
          renderStepsAfterSlider(key, made);
          return made === photo ? undefined : made;
        }, rest.current.image ?? (full ? (editBaseImage.value ?? undefined) : undefined));
      });
    };
  }
  return undefined;
}
