import { previewViewport } from '@/src/lib/editor/preview-coordinates';
import { previewBox$, previewRotationValue, previewScaleValue, previewTranslateValue } from '@/src/state/gestures';
import { previewSize, previewSourceImage$ } from '@/src/state/preview';
import { editBaseImage$ } from '@/src/state/session';
import { ImagePreview, type AbraImage } from '@alakazam/mobile';
import { useEffect, useRef, useState } from 'react';
import { combineLatest } from 'rxjs';

/**
 * The preview of the source image on the native view under the live one; Vessel draws it at the view's size. One preview is open
 * over the edit base for as long as it is the same photo. It shares an image of its own, and each new source is put in that
 * image, so the photo stays on the GPU and a picture that is only the photo with effects over it costs the effects. A new
 * photo opens a new preview.
 */
export function useBasePreview() {
  const [view, setView] = useState<ImagePreview | null>(null);
  // The base preview that is alive now. State is a render behind it, and a viewport sent to one that was just let go of fails.
  const ref = useRef<ImagePreview | null>(null);

  useEffect(() => {
    let current: ImagePreview | null = null;
    let display: AbraImage | null = null;
    let over: unknown = null;
    const release = () => {
      ref.current = null;
      current?.uniffiDestroy();
      display?.uniffiDestroy();
      current = null;
      display = null;
      over = null;
      setView(null);
    };
    const subscription = combineLatest([previewSourceImage$, previewBox$, editBaseImage$]).subscribe(([source, size, base]) => {
      if (!source || !size || !base) return release();
      if (over !== base || !current || !display) {
        release();
        display = source.clone() as AbraImage;
        current = new ImagePreview(display);
        // It draws its first frame as soon as it is shown, so it starts on the part of the photo the box shows (the photo contained,
        // as zoomed and turned now), not on the whole photo stretched to the box.
        current.setViewport(
          ...previewViewport({ width: source.width(), height: source.height() }, size, previewScaleValue.value, previewTranslateValue.value),
        );
        current.setRotation(previewRotationValue.value);
        over = base;
        ref.current = current;
        setView(current);
      } else {
        display.replaceWith(source);
        current.refresh();
      }
      previewSize.next({ width: source.width(), height: source.height() });
    });
    return () => {
      subscription.unsubscribe();
      release();
      previewSize.next(null);
    };
  }, []);

  return { view, ref };
}
