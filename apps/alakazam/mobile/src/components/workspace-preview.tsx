import { usePreviewCamera } from '@/src/hooks/usePreviewCamera';
import { useOwning } from '@/src/lib/native/native-handles';
import {
  ensurePreviewPanResponder,
  previewBox,
  previewMoving,
  previewOrigin,
  previewPanResponder$,
  previewPressed,
  resetPreviewTransform,
  startPreviewGestures,
  trackPreviewTransform,
} from '@/src/state/gestures';
import { useObservable } from 'react-rx';
import { CircleToZoomOverlay } from '@/src/components/editor/view-controls';
import { editorGizmos } from '@/src/state/gizmos';
import { previewCircleMode$ } from '@/src/state/gestures';
import { pluginEditorCanvas, pluginEditorRedraw$ } from '@/src/state/plugin-editor';
import type { CanvasLike } from '@abra/core/canvas';
import { Gizmos, ImagePreview } from '@alakazam/mobile';
import { VesselView } from '@vessel/react-native';
import { useEffect, useRef, useState, type ElementRef } from 'react';
import { StyleSheet, View, type GestureResponderEvent, type LayoutChangeEvent } from 'react-native';
import { auditTime } from 'rxjs';

type Open = { preview: ImagePreview; canvas: CanvasLike };

/** How far a finger may move, in pixels, and how long it may stay down, for a touch to still be a tap. */
const TAP_MOVE = 10;
const TAP_MS = 400;

/**
 * Shows the canvas a plugin's `previewCanvas` returned, contained in the space it is given, the way the editor shows a photo. The
 * canvas is shared, not copied: the Vessel engine composes it on its own thread whenever it has changed, so a plugin that changes its
 * layers (a slider) and says so through `pluginEditorRedraw` draws nothing on the JS thread. The plugin owns the canvas and this
 * only looks at it.
 */
export function WorkspacePreview({ canvas }: { canvas: CanvasLike | null }) {
  const [box, setBox] = useState<{ width: number; height: number } | null>(null);
  const alive = useRef<Open | null>(null);
  const baseViewRef = useRef<ImagePreview | null>(null);
  const areaRef = useRef<ElementRef<typeof View> | null>(null);
  const shown = useRef(canvas);
  shown.current = canvas;
  const [photo, setPhoto] = useState<{ width: number; height: number } | null>(null);
  const panResponder = useObservable(previewPanResponder$, null);
  const circling = useObservable(previewCircleMode$, false);
  // The loop "Circle to zoom" draws, made once and shared with the controls (see `state/gizmos.ts`).
  const gizmos = useOwning(keep => keep(new Gizmos()), []);
  useEffect(() => {
    editorGizmos.next(gizmos);
    return () => editorGizmos.next(null);
  }, [gizmos]);

  const open = useOwning(
    keep => {
      const current = shown.current;
      if (!current || !box) return null;
      // The canvas on show is the newest one the plugin returned. A plugin that returns a new canvas frees the old one first, so for a
      // moment the box can change while this still holds the freed one; the new canvas arrives next and opens then.
      if (current !== pluginEditorCanvas.value) return null;
      try {
        return { preview: keep(ImagePreview.fromCanvas(current)) as ImagePreview, canvas: current };
      } catch (failure) {
        console.warn('The preview canvas was freed before it could be shown:', failure);
        return null;
      }
    },
    [canvas, box?.width, box?.height],
    alive,
  );
  baseViewRef.current = open?.preview ?? null;

  // The size of the canvas on show, which the camera fits to the box.
  useEffect(() => {
    setPhoto(canvas ? { width: canvas.width(), height: canvas.height() } : null);
  }, [canvas]);

  // The same pinch, pan and turn as the editor's photo: the gestures move the camera, the camera moves the views' viewport.
  usePreviewCamera({ photo, liveShown: null, box, baseView: open?.preview ?? null, liveView: null, baseViewRef });
  useEffect(() => {
    ensurePreviewPanResponder();
    const tracking = trackPreviewTransform();
    tracking.add(startPreviewGestures());
    return () => {
      tracking.unsubscribe();
      resetPreviewTransform();
      previewMoving.next(false);
    };
  }, []);
  useEffect(() => {
    previewBox.next(box);
    return () => previewBox.next(null);
  }, [box]);

  // Looking again at the canvas: the plugin changed it in place. The canvas can have a new size, so the camera fits it again.
  useEffect(() => {
    const redraws = pluginEditorRedraw$.pipe(auditTime(16)).subscribe(() => {
      const current = alive.current;
      // A plugin that returns a new canvas frees the one before it, and React draws the new one a moment later, so until then the open
      // preview holds a canvas that is gone. The state already has the new one, so it is the state that says whether the open one is
      // still the canvas on show; its size is not asked of a canvas that is not.
      if (!current || !box || current.canvas !== pluginEditorCanvas.value) return;
      const size = { width: current.canvas.width(), height: current.canvas.height() };
      setPhoto(previous => (previous && previous.width === size.width && previous.height === size.height ? previous : size));
      current.preview.refresh();
    });
    return () => redraws.unsubscribe();
  }, [box]);

  // A tap on the preview is told to the plugin (`app.events.taps`), as the editor's preview does for its photo: one finger that goes down and
  // comes up without moving much or staying long. A second finger, a pan or a pinch is not a tap.
  const tap = useRef<{ x: number; y: number; time: number } | null>(null);
  const onTouchStart = (event: GestureResponderEvent) => {
    const { touches, pageX, pageY } = event.nativeEvent;
    tap.current = touches.length === 1 ? { x: pageX, y: pageY, time: Date.now() } : null;
  };
  const onTouchMove = (event: GestureResponderEvent) => {
    const { touches, pageX, pageY } = event.nativeEvent;
    const start = tap.current;
    if (start && (touches.length !== 1 || Math.hypot(pageX - start.x, pageY - start.y) > TAP_MOVE)) tap.current = null;
  };
  const onTouchEnd = () => {
    const start = tap.current;
    tap.current = null;
    if (!start || Date.now() - start.time > TAP_MS) return;
    areaRef.current?.measureInWindow((left, top) => previewPressed.next({ x: start.x - left, y: start.y - top, pageX: start.x, pageY: start.y }));
  };

  const onLayout = (event: LayoutChangeEvent) => {
    const { width, height } = event.nativeEvent.layout;
    areaRef.current?.measureInWindow((left, top) => previewOrigin.next({ x: left, y: top }));
    setBox(previous => (previous && previous.width === width && previous.height === height ? previous : { width, height }));
  };

  return (
    <View
      ref={areaRef}
      style={styles.area}
      onLayout={onLayout}
      onTouchStart={onTouchStart}
      onTouchMove={onTouchMove}
      onTouchEnd={onTouchEnd}
      {...(panResponder?.panHandlers ?? {})}
    >
      {open && box && VesselView ? (
        <VesselView source={open.preview} pointerEvents="none" style={{ width: box.width, height: box.height }} />
      ) : null}
      {gizmos && box && VesselView ? (
        <VesselView source={gizmos} pointerEvents="none" style={{ position: 'absolute', left: 0, top: 0, width: box.width, height: box.height }} />
      ) : null}
      {circling ? <CircleToZoomOverlay /> : null}
    </View>
  );
}

const styles = StyleSheet.create({
  area: { ...StyleSheet.absoluteFill },
});
