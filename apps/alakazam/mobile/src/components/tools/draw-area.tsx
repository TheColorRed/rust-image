// @refresh reset
import type { AbraImageLike } from '@alakazam/mobile';
import { Color } from '@abra/core/color';
import { Brush } from '@abra/core/drawing';
import { useEffect, useMemo, useRef } from 'react';
import { useObservable } from 'react-rx';
import { useOwning } from '@/src/lib/native/native-handles';
import { PanResponder, PixelRatio, StyleSheet, View } from 'react-native';
import { areaTarget, type AreaTarget, type DrawnPoint } from '@/src/state/draw-area';
import { editorGizmos, editorGizmos$ } from '@/src/state/gizmos';
import { computePreviewImagePoint } from '@/src/lib/editor/preview-coordinates';
import { currentImage } from '@/src/state/session';
import { previewBox, previewDrawing, previewRotationValue, previewScaleValue, previewTranslateValue } from '@/src/state/gestures';

export interface DrawAreaOverlayProps {
  /** The name of the outline on the gizmos layer while it is drawn. Tools that can be open together need different names. */
  gizmo: string;
  /** How opaque the outline is while it is drawn, from 0 to 1. */
  opacity?: number;
  /** A loop that encloses less than this share of the photo is a mis-tap and is ignored. */
  minShare?: number;
  /** A finger or pen drew a loop around part of the photo. `outline` is its points in the photo's pixels, in the order they were drawn. */
  onDrawn: (outline: DrawnPoint[]) => void;
}

/**
 * Lets a finger or pen circle part of the photo. It captures one freehand loop at a time, shows it on the gizmos layer while it is
 * drawn, and gives it to `onDrawn` in the photo's pixels, so what the loop means (a color's area, what to remove) is up to the tool.
 * It covers the preview and takes one-finger touches, so mount it only while a tool is drawing. A second finger goes to the preview, so the
 * photo can still be pinched and panned while an area is drawn; the loop in progress is dropped.
 */
export function DrawAreaOverlay({ gizmo, opacity = 0.7, minShare = 0.002, onDrawn }: DrawAreaOverlayProps) {
  const points = useRef<DrawnPoint[]>([]);
  const gizmos = useObservable(editorGizmos$, editorGizmos.value);
  // The responder lives as long as the gizmos do, so the latest callback is read through a ref.
  const drawn = useRef(onDrawn);
  drawn.current = onDrawn;
  const brush = useOwning(
    keep =>
      keep(
        new Brush()
          .withColor(Color.white())
          .withSize(Math.round(3 * PixelRatio.get()))
          .withHardness(1)
          .withOpacity(opacity) as Brush,
      ),
    [opacity],
  );
  useEffect(() => {
    previewDrawing.next(true);
    return () => {
      previewDrawing.next(false);
      gizmos?.remove(gizmo);
    };
  }, [gizmos, gizmo]);

  const responder = useMemo(() => {
    const density = PixelRatio.get();
    const draw = (point: DrawnPoint) => brush && gizmos?.addPoint(gizmo, point.x * density, point.y * density, brush);
    // Where this layer is on the screen, taken from the first touch, which lands on the layer itself.
    const origin = { x: 0, y: 0 };
    // The touch relative to this layer, which covers the preview box, so it matches the box the photo is laid out in. It is the screen
    // position minus the layer's, because `locationX` and `locationY` are relative to whatever view is under the finger, such as a
    // button over the preview. A finger that leaves the box is held at its edge.
    const touch = (event: { nativeEvent: { pageX: number; pageY: number } }) => {
      const box = previewBox.value;
      const x = event.nativeEvent.pageX - origin.x;
      const y = event.nativeEvent.pageY - origin.y;
      return box ? { x: Math.min(Math.max(x, 0), box.width), y: Math.min(Math.max(y, 0), box.height) } : { x, y };
    };
    const finish = () => {
      const loop = points.current;
      points.current = [];
      gizmos?.remove(gizmo);
      const outline = inPhoto(loop);
      if (outline && enclosedShare(outline.points, outline.image) >= minShare) drawn.current(outline.points);
    };
    return PanResponder.create({
      onStartShouldSetPanResponder: () => true,
      onMoveShouldSetPanResponder: () => true,
      // A second finger belongs to the preview's pinch responder, not to the outline.
      onPanResponderTerminationRequest: () => true,
      onPanResponderGrant: event => {
        gizmos?.remove(gizmo);
        origin.x = event.nativeEvent.pageX - event.nativeEvent.locationX;
        origin.y = event.nativeEvent.pageY - event.nativeEvent.locationY;
        points.current = [touch(event)];
        draw(points.current[0]);
      },
      onPanResponderMove: event => {
        if (event.nativeEvent.touches.length > 1) return;
        const point = touch(event);
        points.current.push(point);
        draw(point);
      },
      onPanResponderRelease: finish,
      onPanResponderTerminate: () => {
        points.current = [];
        gizmos?.remove(gizmo);
      },
    });
  }, [gizmos, brush, gizmo, minShare]);

  return <View style={StyleSheet.absoluteFill} {...responder.panHandlers} />;
}

/** A loop drawn on the preview, in the open photo's pixels. Points off the photo are pulled to its edge, so the loop follows the edge instead of cutting straight across. `null` when the photo is not open or the loop has fewer than 3 points over it. */
function inPhoto(screenPoints: readonly DrawnPoint[]) {
  const image = currentImage.value as AbraImageLike | null;
  const box = previewBox.value;
  if (!image || !box || screenPoints.length < 3) return null;
  const width = image.width();
  const height = image.height();
  const points: DrawnPoint[] = [];
  for (const point of screenPoints) {
    const inImage = computePreviewImagePoint(
      width,
      height,
      box,
      previewScaleValue.value,
      previewTranslateValue.value,
      point,
      previewRotationValue.value,
      true,
    );
    if (inImage) points.push(inImage);
  }
  return points.length < 3 ? null : { points, image: { width, height } };
}

/** The share of the photo that a loop encloses, from 0 to 1. */
function enclosedShare(points: readonly DrawnPoint[], image: { width: number; height: number }) {
  const twice = points.reduce((sum, point, index) => {
    const next = points[(index + 1) % points.length];
    return sum + point.x * next.y - next.x * point.y;
  }, 0);
  return Math.abs(twice / 2) / (image.width * image.height);
}

/**
 * Offers the area that a control lets the person draw, while the control is open (see `AreaTarget`). Pass `null` when it offers none.
 * `target` should be memoized, because the buttons and the outline are updated each time it changes.
 */
export function useAreaTarget(owner: string, target: Omit<AreaTarget, 'owner'> | null) {
  useEffect(() => {
    if (target) areaTarget.next({ ...target, owner });
    else if (areaTarget.value?.owner === owner) areaTarget.next(null);
  }, [owner, target]);
  useEffect(
    () => () => {
      if (areaTarget.value?.owner === owner) areaTarget.next(null);
    },
    [owner],
  );
}
