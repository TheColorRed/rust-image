import { ImagePreview, type AbraImage } from '@alakazam/mobile';
import { useCallback, useRef, useState } from 'react';
import { useOwning } from '@/src/lib/native/native-handles';

export interface LiveSessionControls {
  /** Size in pixels of the image shown on the native view (only its proportions matter), or null until the session is open. */
  size: { width: number; height: number } | null;
  /** What to show on a `VesselView`, or null until the session is open. */
  view: ImagePreview | null;
  /** Any error from opening the session. */
  error: string | null;
  /** Shows `photo` in the view in place of the image, until another is shown. Rust decides when to draw; this never waits for a frame. */
  show: (photo: AbraImage) => void;
}

interface Live {
  display: AbraImage;
  session: ImagePreview;
  size: { width: number; height: number };
}

/**
 * Keeps one live session open on `image` for as long as `image` is given. Give its `view` to a `VesselView`. Rust owns
 * the engine, the state and the schedule: the screen only says what to show. The session shares a copy of `image` that `show`
 * puts pictures in, so the image itself is never changed. It is closed when `image` goes away or the component unmounts.
 */
export function useLiveSession(image: AbraImage | null): LiveSessionControls {
  const [error, setError] = useState<string | null>(null);
  const alive = useRef<Live | null>(null);
  const live = useOwning(
    keep => {
      if (!image) return null;
      try {
        const display = keep(image.clone() as AbraImage);
        const session = keep(new ImagePreview(display));
        setError(null);
        return { display, session, size: { width: image.width(), height: image.height() } };
      } catch (e: any) {
        setError(String(e?.message ?? e));
        return null;
      }
    },
    [image],
    alive,
  );

  const show = useCallback((photo: AbraImage) => {
    alive.current?.display.replaceWith(photo);
    alive.current?.session.refresh();
  }, []);

  return { size: live?.size ?? null, view: live?.session ?? null, error, show };
}
