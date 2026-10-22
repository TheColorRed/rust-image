import { previewViewport } from '@/src/lib/editor/preview-coordinates';
import { useOwning } from '@/src/lib/native/native-handles';
import { previewBox, previewBox$, previewRotationValue, previewScaleValue, previewTranslateValue } from '@/src/state/gestures';
import { previewSize, previewSourceImage, previewSourceImage$ } from '@/src/state/preview';
import { editBaseImage, editBaseImage$ } from '@/src/state/session';
import { ImagePreview, type AbraImage } from '@alakazam/mobile';
import { useEffect, useMemo, useRef } from 'react';
import { useObservable } from 'react-rx';
import { skip } from 'rxjs/operators';

interface Base {
  preview: ImagePreview;
  display: AbraImage;
}

/**
 * The preview of the source image on the native view under the live one; Vessel draws it at the view's size. One preview is open
 * over the edit base for as long as it is the same photo. It shares an image of its own, and each new source is put in that
 * image, so the photo stays on the GPU and a picture that is only the photo with effects over it costs the effects. A new
 * photo opens a new preview.
 */
export function useBasePreview() {
  const source = useObservable(previewSourceImage$, previewSourceImage.value);
  const box = useObservable(previewBox$, previewBox.value);
  const base = useObservable(editBaseImage$, editBaseImage.value);
  const ready = !!source && !!box && !!base;
  // The base preview that is alive now. State is a render behind it, and a viewport sent to one that was just let go of fails.
  const alive = useRef<Base | null>(null);
  const open = useOwning(
    keep => {
      const photo = previewSourceImage.value;
      const shown = previewBox.value;
      if (!ready || !photo || !shown) return null;
      const display = keep(photo.clone() as AbraImage);
      const preview = keep(new ImagePreview(display));
      // It draws its first frame as soon as it is shown, so it starts on the part of the photo the box shows (the photo contained,
      // as zoomed and turned now), not on the whole photo stretched to the box.
      preview.setViewport(
        ...previewViewport({ width: photo.width(), height: photo.height() }, shown, previewScaleValue.value, previewTranslateValue.value),
      );
      preview.setRotation(previewRotationValue.value);
      previewSize.next({ width: photo.width(), height: photo.height() });
      // A new source of the same photo goes into the preview's own image. It starts listening as the preview is made, so a source that
      // arrives before the next render is not missed, and it has to act as the source is emitted: the editor frees the image it
      // replaced, so one read later may already be freed. The first emission replays the source it was just cloned from.
      keep(
        previewSourceImage$.pipe(skip(1)).subscribe(next => {
          if (!next) return;
          display.replaceWith(next);
          preview.refresh();
          previewSize.next({ width: next.width(), height: next.height() });
        }),
      );
      return { preview, display };
    },
    [ready, base],
    alive,
  );
  useEffect(() => () => previewSize.next(null), []);

  const ref = useMemo(
    () => ({
      get current() {
        return alive.current?.preview ?? null;
      },
    }),
    [],
  );
  return { view: open?.preview ?? null, ref };
}
