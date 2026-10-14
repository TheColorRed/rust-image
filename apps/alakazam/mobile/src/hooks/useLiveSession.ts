import { ImagePreview, type AbraImage } from '@alakazam/mobile';
import { useCallback, useEffect, useRef, useState } from 'react';

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

/**
 * Keeps one live session open on `image` for as long as `image` is given. Give its `view` to a `VesselView`. Rust owns
 * the engine, the state and the schedule: the screen only says what to show. The session shares a copy of `image` that `show`
 * puts pictures in, so the image itself is never changed. It is closed when `image` goes away or the component unmounts.
 */
export function useLiveSession(image: AbraImage | null): LiveSessionControls {
  const [size, setSize] = useState<{ width: number; height: number } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [view, setView] = useState<ImagePreview | null>(null);
  const sessionRef = useRef<ImagePreview | null>(null);
  const displayRef = useRef<AbraImage | null>(null);

  useEffect(() => {
    if (!image) return;
    try {
      const display = image.clone() as AbraImage;
      const session = new ImagePreview(display);
      displayRef.current = display;
      sessionRef.current = session;
      setView(session);
      setSize({ width: image.width(), height: image.height() });
      setError(null);
    } catch (e: any) {
      setError(String(e?.message ?? e));
    }
    return () => {
      sessionRef.current?.uniffiDestroy();
      displayRef.current?.uniffiDestroy();
      sessionRef.current = null;
      displayRef.current = null;
      setView(null);
      setSize(null);
    };
  }, [image]);

  const show = useCallback((photo: AbraImage) => {
    displayRef.current?.replaceWith(photo);
    sessionRef.current?.refresh();
  }, []);

  return { size, view, error, show };
}
