import { ImagePreview, type AbraImage, type Message } from '@alakazam/mobile';
import { useCallback, useEffect, useRef, useState } from 'react';

export interface LiveSessionControls {
  /** Size in pixels of the image shown on the native view (only its proportions matter), or null until the session is open. */
  size: { width: number; height: number } | null;
  /** What to show on a `VesselView`, or null until the session is open. */
  view: ImagePreview | null;
  /** Any error from opening the session. */
  error: string | null;
  /** Tells a view in the session what changed. Rust decides when to draw; this never waits for a frame. */
  send: (message: Message) => void;
}

/**
 * Keeps one live session open on `image` for as long as `image` is given. Give its `view` to a `VesselView`. Rust owns
 * the engine, the state and the schedule: the screen only sends what changed. The session is closed when `image` goes
 * away or the component unmounts. Edits to the image show up on their own.
 */
export function useLiveSession(image: AbraImage | null): LiveSessionControls {
  const [size, setSize] = useState<{ width: number; height: number } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [view, setView] = useState<ImagePreview | null>(null);
  const sessionRef = useRef<ImagePreview | null>(null);

  useEffect(() => {
    if (!image) return;
    try {
      const session = new ImagePreview(image);
      sessionRef.current = session;
      setView(session);
      setSize({ width: image.width(), height: image.height() });
      setError(null);
    } catch (e: any) {
      setError(String(e?.message ?? e));
    }
    return () => {
      sessionRef.current?.uniffiDestroy();
      sessionRef.current = null;
      setView(null);
      setSize(null);
    };
  }, [image]);

  const send = useCallback((message: Message) => {
    sessionRef.current?.send(message);
  }, []);

  return { size, view, error, send };
}
