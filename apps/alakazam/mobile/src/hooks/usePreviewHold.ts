import { restorePreviewRequested, showCheckpointRequested } from '@/src/state/commands';
import { previewOrigin, previewPressed } from '@/src/state/gestures';
import { useCallback, useEffect, useRef, type ElementRef, type RefObject } from 'react';
import type { GestureResponderEvent, Pressable } from 'react-native';

const CHECKPOINT_HOLD_MS = 250;
/** How far (screen dp) a finger may drift and still count as holding still. */
const HOLD_MOVE_TOLERANCE = 8;

/**
 * Press handlers for the photo: holding a finger still shows the before/after preview until it lifts or moves, and a quick
 * press is a tap on the photo.
 */
export function usePreviewHold(touchTarget: RefObject<ElementRef<typeof Pressable> | null>) {
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const previewActive = useRef(false);
  // Where the finger went down, while a hold might still happen.
  const start = useRef<{ x: number; y: number } | null>(null);
  // Set when a touch ended a hold, whose release is not a tap on the photo.
  const ended = useRef(false);

  const clearTimer = useCallback(() => {
    if (timer.current) clearTimeout(timer.current);
    timer.current = null;
  }, []);

  // Takes down a before/after preview that is showing.
  const endPreview = useCallback(() => {
    if (!previewActive.current) return;
    previewActive.current = false;
    ended.current = true;
    restorePreviewRequested.next();
  }, []);

  const onPressIn = useCallback(
    (event: GestureResponderEvent) => {
      start.current = { x: event.nativeEvent.pageX, y: event.nativeEvent.pageY };
      // Re-measure on every touch-down: a layout-time measurement can be stale (screen transition,
      // insets settling), which showed up as the zoom drifting vertically.
      touchTarget.current?.measureInWindow((left, top) => previewOrigin.next({ x: left, y: top }));
      clearTimer();
      previewActive.current = false;
      ended.current = false;
      timer.current = setTimeout(() => {
        timer.current = null;
        previewActive.current = true;
        showCheckpointRequested.next();
      }, CHECKPOINT_HOLD_MS);
    },
    [touchTarget, clearTimer],
  );

  // A finger that moves is drawing, panning or dragging, not holding: the before/after preview is only for a finger that stays put,
  // so moving ends a hold that has not shown yet, and takes down one that has.
  const onTouchMove = useCallback(
    (event: GestureResponderEvent) => {
      const origin = start.current;
      if (!origin || Math.hypot(event.nativeEvent.pageX - origin.x, event.nativeEvent.pageY - origin.y) <= HOLD_MOVE_TOLERANCE) return;
      start.current = null;
      clearTimer();
      endPreview();
    },
    [clearTimer, endPreview],
  );

  const onPressOut = useCallback(() => {
    start.current = null;
    clearTimer();
    endPreview();
  }, [clearTimer, endPreview]);

  const onPress = useCallback(
    (event: GestureResponderEvent) => {
      if (ended.current) return;
      const { pageX, pageY } = event.nativeEvent;
      // locationX/Y can be relative to the transformed photo or outline child that was hit.
      touchTarget.current?.measureInWindow((left, top) => {
        previewPressed.next({ x: pageX - left, y: pageY - top, pageX, pageY });
      });
    },
    [touchTarget],
  );

  // Leaving the screen takes down a preview that is still showing.
  useEffect(
    () => () => {
      clearTimer();
      endPreview();
    },
    [clearTimer, endPreview],
  );

  return { onPressIn, onPressOut, onTouchMove, onPress };
}
