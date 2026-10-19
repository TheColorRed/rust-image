import { previewBox, previewLayoutFrame, previewOrigin, type PreviewBox } from '@/src/state/gestures';
import { useCallback, useEffect, useRef, type ElementRef, type RefObject } from 'react';
import type { LayoutChangeEvent, Pressable } from 'react-native';

// How long the photo takes to ease to a new height when the panel changes size.
const BOX_TWEEN_MS = 240;

/**
 * The photo always fits the visible area, between the top bar and the panel. When the panel changes height the area changes too,
 * so the box eases to its new height over a few frames instead of jumping. Returns the `onLayout` handler of that area.
 */
export function usePreviewBox(touchTarget: RefObject<ElementRef<typeof Pressable> | null>) {
  const target = useRef<PreviewBox | null>(null);
  const tween = useRef<number | null>(null);

  useEffect(
    () => () => {
      if (tween.current !== null) cancelAnimationFrame(tween.current);
      if (previewLayoutFrame.value !== null) cancelAnimationFrame(previewLayoutFrame.value);
      previewLayoutFrame.next(null);
    },
    [],
  );

  const tweenTo = useCallback((to: PreviewBox) => {
    const from = previewBox.value;
    if (tween.current !== null) cancelAnimationFrame(tween.current);
    tween.current = null;
    // A first layout or a turned phone (new width) is a new box, not a resize, so it is not animated.
    if (!from || from.width !== to.width) {
      previewBox.next(to);
      return;
    }
    const started = Date.now();
    const step = () => {
      const t = Math.min(1, (Date.now() - started) / BOX_TWEEN_MS);
      if (t >= 1) {
        tween.current = null;
        previewBox.next(to);
        return;
      }
      const eased = 1 - (1 - t) ** 3;
      previewBox.next({ width: to.width, height: Math.round(from.height + (to.height - from.height) * eased) });
      tween.current = requestAnimationFrame(step);
    };
    tween.current = requestAnimationFrame(step);
  }, []);

  // Debounced to a frame: a layout that shifts while an edit replay is in flight would otherwise
  // land two rAF-deferred state updates in one native frame batch.
  return useCallback(
    (event: LayoutChangeEvent) => {
      const { width, height } = event.nativeEvent.layout;
      if (previewLayoutFrame.value !== null) cancelAnimationFrame(previewLayoutFrame.value);
      previewLayoutFrame.next(
        requestAnimationFrame(() => {
          previewLayoutFrame.next(null);
          touchTarget.current?.measureInWindow((left, top) => previewOrigin.next({ x: left, y: top }));
          if (target.current?.width === width && target.current.height === height) return;
          target.current = { width, height };
          tweenTo(target.current);
        }),
      );
    },
    [touchTarget, tweenTo],
  );
}
