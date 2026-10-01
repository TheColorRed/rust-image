import { AbraLiveImage, type AbraImage, type EffectSpec } from '@alakazam/mobile';
import { AlphaType, ColorType, Skia, type SkImage } from '@shopify/react-native-skia';
import { useCallback, useEffect, useRef, useState } from 'react';

/** Skia may still be drawing a replaced image for a frame or two; disposing it at once throws. */
export function disposeLater(p_image: SkImage | null | undefined) {
  if (p_image) setTimeout(() => p_image.dispose(), 1000);
}

/** Display ticks with no new frame before the poll loop goes idle again. */
const IDLE_TICKS = 30;

export interface LivePreviewControls {
  /** The newest rendered frame, or null before the first effect is applied. */
  frame: SkImage | null;
  /** Whether frames are rendered on the GPU (otherwise on the CPU). */
  isGpu: boolean;
  /** Size in pixels of the frames drawn on the native surface, or null while frames come back through `frame`. */
  surface: { width: number; height: number } | null;
  /** Any error from creating or rendering the preview. */
  error: string | null;
  /**
   * Applies an effect under `key`, which defaults to the kind of effect. The first call with a key adds the effect to
   * the end of the chain; later calls with the same key replace it in place, so call it on every slider change. Use
   * different keys to apply the same kind of effect more than once.
   */
  apply: (effect: EffectSpec, key?: string) => void;
  /** Whether an effect is applied under `key` in the current preview. A new preview starts with none. */
  has: (key: string) => boolean;
  /** Removes the effect applied under `key`. */
  remove: (key: string) => void;
  /** Removes every effect. */
  clear: () => void;
}

/**
 * Renders effects over a screen-sized copy of `image` while a control is dragged. `apply` returns immediately on the
 * GPU; frames are collected on display ticks, and only the newest one is ever shown. Bump `revision` after the image itself changes to restart from its new pixels.
 */
export function useLivePreview(
  image: AbraImage | null,
  maxWidth: number,
  maxHeight: number,
  revision = 0,
  surfaceId?: number,
): LivePreviewControls {
  const [frame, setFrame] = useState<SkImage | null>(null);
  const [isGpu, setIsGpu] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [surface, setSurface] = useState<{ width: number; height: number } | null>(null);
  const previewRef = useRef<AbraLiveImage | null>(null);
  /** The chain id of each effect, by the key it was applied under. */
  const idsRef = useRef(new Map<string, number>());
  const rafRef = useRef<number | null>(null);
  const idleRef = useRef(0);

  const stop = useCallback(() => {
    if (rafRef.current !== null) cancelAnimationFrame(rafRef.current);
    rafRef.current = null;
  }, []);

  const tick = useCallback(() => {
    rafRef.current = null;
    const preview = previewRef.current;
    if (!preview) return;
    try {
      const pixels = preview.poll();
      if (pixels) {
        idleRef.current = 0;
        const next = Skia.Image.MakeImage(
          { width: pixels.width, height: pixels.height, colorType: ColorType.RGBA_8888, alphaType: AlphaType.Unpremul },
          Skia.Data.fromBytes(new Uint8Array(pixels.data)),
          pixels.width * 4,
        );
        setFrame((previous) => {
          disposeLater(previous);
          return next;
        });
      } else {
        idleRef.current += 1;
      }
    } catch (e: any) {
      setError(String(e?.message ?? e));
      return;
    }
    if (idleRef.current < IDLE_TICKS) rafRef.current = requestAnimationFrame(tick);
  }, []);

  const wake = useCallback(() => {
    idleRef.current = 0;
    if (rafRef.current === null) rafRef.current = requestAnimationFrame(tick);
  }, [tick]);

  useEffect(() => {
    if (!image) return;
    try {
      const preview = new AbraLiveImage(image, maxWidth, maxHeight);
      previewRef.current = preview;
      idsRef.current.clear();
      if (surfaceId !== undefined) preview.attachSurface(surfaceId);
      setIsGpu(preview.isGpu());
      setError(null);
    } catch (e: any) {
      setError(String(e?.message ?? e));
    }
    return () => {
      stop();
      previewRef.current?.uniffiDestroy();
      previewRef.current = null;
      setSurface(null);
      setFrame((previous) => {
        disposeLater(previous);
        return null;
      });
    };
  }, [image, maxWidth, maxHeight, revision, surfaceId, stop]);

  const apply = useCallback(
    (effect: EffectSpec, key: string = effect.tag) => {
      const preview = previewRef.current;
      if (!preview) return;
      try {
        const id = idsRef.current.get(key);
        if (id === undefined) idsRef.current.set(key, preview.addEffect(effect));
        else preview.setEffect(id, effect);
        if (surfaceId !== undefined && preview.isPresenting()) {
          setSurface(current =>
            current?.width === preview.width() && current.height === preview.height()
              ? current
              : { width: preview.width(), height: preview.height() },
          );
        }
        wake();
      } catch (e: any) {
        setError(String(e?.message ?? e));
      }
    },
    [wake, surfaceId],
  );

  const has = useCallback((key: string) => idsRef.current.has(key), []);

  const remove = useCallback(
    (key: string) => {
      const id = idsRef.current.get(key);
      if (id === undefined) return;
      idsRef.current.delete(key);
      previewRef.current?.removeEffect(id);
      wake();
    },
    [wake],
  );

  const clear = useCallback(() => {
    idsRef.current.clear();
    previewRef.current?.clear();
    wake();
  }, [wake]);

  return { frame, isGpu, surface, error, apply, has, remove, clear };
}
