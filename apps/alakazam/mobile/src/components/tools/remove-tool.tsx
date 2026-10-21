import type { AbraImageLike } from '@alakazam/mobile';
import { Color } from '@abra/core/color';
import { Brush } from '@abra/core/drawing';
import { useEffect, useMemo, useRef } from 'react';
import { useObservable } from 'react-rx';
import { PanResponder, PixelRatio, Pressable, StyleSheet, Text, View } from 'react-native';
import { BehaviorSubject } from 'rxjs';
import { editorGizmos, editorGizmos$ } from '@/src/state/gizmos';
import { useTheme, type ThemeColors } from '@/src/lib/ui/theme';
import { computePreviewImagePoint } from '@/src/lib/editor/preview-coordinates';
import { removeSelection as applySelection } from '@/src/lib/tools/remove';
import { commitRemove } from '@/src/lib/editor/editor-history';
import { removeApplyRequested$ } from '@/src/state/commands';
import { isRemoveToolFocused$ } from '@/src/state/edits';
import { historyReady } from '@/src/state/project-session';
import { flushFullReplay } from '@/src/lib/editor/editor-replay';
import { currentImage } from '@/src/state/session';
import {
  previewBox,
  previewCircling,
  previewOrigin,
  previewRotationValue,
  previewScaleValue,
  previewTranslateValue,
} from '@/src/state/gestures';

/** The current closed selection, in full-size image pixels. It is discarded when the tool closes. */
export const removeSelectionPoints = new BehaviorSubject<[number, number][] | null>(null);
export const removeSelectionPoints$ = removeSelectionPoints.asObservable();

const MIN_SELECTION_AREA = 64;

/** The controls beneath a freehand removal: redraw to replace the loop, then remove what it contains. */
export function RemoveControls({ onApply }: { onApply: () => void }) {
  const { colors } = useTheme();
  const styles = useMemo(() => createStyles(colors), [colors]);
  const selection = useObservable(removeSelectionPoints$, removeSelectionPoints.value);
  return (
    <View style={styles.controlRow}>
      <View style={styles.topRow}>
        <Text style={styles.hint}>
          {selection ? 'Draw again to change the outline' : 'Draw around what you want to remove'}
        </Text>
      </View>
      <Pressable
        style={[styles.applyButton, !selection && styles.applyDisabled]}
        onPress={onApply}
        disabled={!selection}
      >
        <Text style={styles.applyButtonText}>Remove</Text>
      </Pressable>
    </View>
  );
}

/** Applies a selected polygon on the full-size image and gives it a replayable history step. */
export function RemoveToolController() {
  useEffect(() => {
    const subscriptions = [
      removeApplyRequested$.subscribe(() => {
        const points = removeSelectionPoints.value;
        if (!points || !historyReady.value) return;
        flushFullReplay();
        const image = currentImage.value;
        if (!image) return;
        applySelection(image, points);
        commitRemove(points);
        removeSelectionPoints.next(null);
        editorGizmos.value?.remove('remove-selection');
      }),
      isRemoveToolFocused$.subscribe(focused => {
        if (focused) return;
        removeSelectionPoints.next(null);
        editorGizmos.value?.remove('remove-selection');
      }),
    ];
    return () => subscriptions.forEach(subscription => subscription.unsubscribe());
  }, []);
  return null;
}

/** Captures one freehand loop over the preview and draws it as a semi-transparent brush line on the gizmos layer. */
export function RemoveSelectionOverlay() {
  const points = useRef<{ x: number; y: number }[]>([]);
  const drawing = useRef(false);
  const gizmos = useObservable(editorGizmos$, editorGizmos.value);
  const brush = useMemo(() => {
    const density = PixelRatio.get();
    return new Brush()
      .withColor(Color.white())
      .withSize(Math.round(3 * density))
      .withHardness(1)
      .withOpacity(0.55) as Brush;
  }, []);
  useEffect(() => {
    previewCircling.next(true);
    return () => {
      previewCircling.next(false);
      gizmos?.remove('remove-selection');
      brush.uniffiDestroy();
    };
  }, [brush, gizmos]);

  const draw = (point: { x: number; y: number }) => {
    const density = PixelRatio.get();
    gizmos?.addPoint('remove-selection', point.x * density, point.y * density, brush);
  };
  const responder = useMemo(
    () =>
      PanResponder.create({
        onStartShouldSetPanResponder: () => true,
        onMoveShouldSetPanResponder: () => true,
        // A second finger belongs to the preview's pinch responder, not to the outline.
        onPanResponderTerminationRequest: () => true,
        onPanResponderGrant: event => {
          const point = {
            x: event.nativeEvent.pageX - previewOrigin.value.x,
            y: event.nativeEvent.pageY - previewOrigin.value.y,
          };
          points.current = [point];
          drawing.current = false;
        },
        onPanResponderMove: event => {
          if (event.nativeEvent.touches.length > 1) return;
          const point = {
            x: event.nativeEvent.pageX - previewOrigin.value.x,
            y: event.nativeEvent.pageY - previewOrigin.value.y,
          };
          if (!drawing.current) {
            drawing.current = true;
            gizmos?.remove('remove-selection');
            removeSelectionPoints.next(null);
            draw(points.current[0]);
          }
          points.current.push(point);
          draw(point);
        },
        onPanResponderRelease: () => {
          if (drawing.current) finishSelection(points.current, draw, () => gizmos?.remove('remove-selection'));
        },
        onPanResponderTerminate: () => {
          if (!drawing.current) return;
          gizmos?.remove('remove-selection');
          removeSelectionPoints.next(null);
        },
      }),
    [gizmos, brush],
  );

  return <View style={StyleSheet.absoluteFill} {...responder.panHandlers} />;
}

function finishSelection(
  screenPoints: readonly { x: number; y: number }[],
  draw: (point: { x: number; y: number }) => void,
  clear: () => void,
) {
  const image = currentImage.value as AbraImageLike | null;
  const box = previewBox.value;
  if (!image || !box || screenPoints.length < 3) {
    clear();
    return removeSelectionPoints.next(null);
  }
  const points = screenPoints.map(point =>
    computePreviewImagePoint(
      image.width(),
      image.height(),
      box,
      previewScaleValue.value,
      previewTranslateValue.value,
      point,
      previewRotationValue.value,
    ),
  );
  if (points.some(point => !point)) {
    clear();
    return removeSelectionPoints.next(null);
  }
  const polygon = points as { x: number; y: number }[];
  const area = Math.abs(
    polygon.reduce((sum, point, index) => {
      const next = polygon[(index + 1) % polygon.length];
      return sum + point.x * next.y - next.x * point.y;
    }, 0) / 2,
  );
  if (area < MIN_SELECTION_AREA) {
    clear();
    return removeSelectionPoints.next(null);
  }
  // Close the visual loop after a valid release; Area.fromPoints closes its own outline for the actual repair.
  draw(screenPoints[0]);
  removeSelectionPoints.next(polygon.map(point => [point.x, point.y]));
}

function createStyles(colors: ThemeColors) {
  return StyleSheet.create({
    controlRow: { alignItems: 'center', gap: 8, paddingHorizontal: 16 },
    topRow: { flexDirection: 'row', alignItems: 'center', gap: 10, alignSelf: 'stretch' },
    hint: { flex: 1, color: colors.textSecondary, fontSize: 12 },
    applyButton: { paddingHorizontal: 20, paddingVertical: 10, borderRadius: 18, backgroundColor: colors.accent },
    applyDisabled: { opacity: 0.45 },
    applyButtonText: { color: '#fff', fontSize: 14, fontWeight: '700' },
  });
}
