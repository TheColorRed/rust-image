import { type AbraImage, type EffectSpec } from '@alakazam/mobile';
import { useEffect, useState } from 'react';
import { PixelRatio } from 'react-native';
import { skip } from 'rxjs/operators';
import { useObservable } from 'react-rx';
import { useLivePreview } from '@/src/hooks/useLivePreview';
import { type ActionControl } from '@/src/lib/edit-sections';
import { renderPreviewForAction } from '@/src/lib/editor-replay';
import { actionPreviewRequested$ } from '@/src/state/commands';
import { previewBox$ } from '@/src/state/gestures';
import { liveFrame, previewSourceImage$ } from '@/src/state/preview';

/** How long the preview stays over the committed image, so the hand-off between them can't flash. */
const PREVIEW_HANDOFF_MS = 150;

/**
 * Instant preview for tapping an action that has a live form. The tap shows the effect on the GPU at viewport size
 * right away; the full-resolution replay that follows replaces the frame once it is done, as after a slider release.
 */
export function useLiveAction() {
  const box = useObservable(previewBox$, null);
  const [request, setRequest] = useState<{
    control: ActionControl;
    image: AbraImage;
    chain: { key: string; effect: EffectSpec }[] | null;
  } | null>(null);

  useEffect(() => {
    const subscription = actionPreviewRequested$.subscribe(control => {
      const preview = renderPreviewForAction(control);
      if (preview) setRequest({ control, ...preview });
    });
    return () => subscription.unsubscribe();
  }, []);

  // Once the replay has committed the real result (or undo/redo swapped the image), the preview's job is done. Left in
  // place its frame or native surface would keep covering the photo, so undo would appear to do nothing.
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    const subscription = previewSourceImage$.pipe(skip(1)).subscribe(() => {
      // Skia draws the committed image a frame or two after it is set. Taking the preview away sooner shows the
      // un-edited photo for an instant, which is very visible on a big change such as Invert. The preview matches the
      // committed image, so leaving it up a little longer costs nothing.
      clearTimeout(timer);
      timer = setTimeout(() => setRequest(null), PREVIEW_HANDOFF_MS);
    });
    return () => {
      subscription.unsubscribe();
      clearTimeout(timer);
    };
  }, []);

  // The request owns its image; free it once it is replaced or the editor closes.
  const image = request?.image ?? null;
  useEffect(() => () => image?.uniffiDestroy(), [image]);

  const ratio = PixelRatio.get();
  // No native surface: an action needs a single frame, and creating and hiding a surface view on every tap can flash.
  const preview = useLivePreview(image, Math.round((box?.width ?? 0) * ratio), Math.round((box?.height ?? 0) * ratio));

  // Declared after useLivePreview so the preview exists by the time its effect is applied.
  const { apply } = preview;
  useEffect(() => {
    if (!request) return;
    if (request.chain) {
      for (const step of request.chain) apply(step.effect, step.key);
      return;
    }
    const spec = request.control.live?.();
    if (spec) apply(spec);
  }, [request, apply]);

  useEffect(() => {
    liveFrame.next(preview.frame);
    return () => liveFrame.next(null);
  }, [preview.frame]);
}
