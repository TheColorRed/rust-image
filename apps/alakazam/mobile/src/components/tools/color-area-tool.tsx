import { polygonDifference, polygonUnion } from '@abra/core/geometry';
import type { AbraImageLike } from '@alakazam/mobile';
import { Color } from '@abra/core/color';
import { Brush } from '@abra/core/drawing';
import { useEffect, useMemo, useRef } from 'react';
import { useObservable } from 'react-rx';
import { PanResponder, PixelRatio, StyleSheet, View } from 'react-native';
import Svg, { Path } from 'react-native-svg';
import { BehaviorSubject } from 'rxjs';
import { editorGizmos, editorGizmos$ } from '@/src/state/gizmos';
import { computePreviewImagePoint } from '@/src/lib/editor/preview-coordinates';
import { areaPoints, type AdjustmentValue, encodeAreaRings, findSection, leafControls } from '@/src/lib/project/edit-sections';
import { commitColorArea } from '@/src/lib/editor/editor-history';
import { activeSectionKey, activeSectionKey$, adjustments, adjustments$, areaOutlinesHidden, areaOutlinesHidden$, openSliderState, openSliderState$ } from '@/src/state/edits';
import { currentImage } from '@/src/state/session';
import {
  previewBox,
  previewCircling,
  previewRotationValue,
  previewScaleSettled$,
  previewScaleValue,
  previewTranslateValue,
  type PreviewBox,
} from '@/src/state/gestures';

/** The adjustment key of the area being drawn, or null when the drawing is closed. The gesture bar's Draw area button sets it. */
export const areaDrawKey = new BehaviorSubject<string | null>(null);
export const areaDrawKey$ = areaDrawKey.asObservable();
/** Whether a drawn loop adds to the area being drawn or takes away from it. The gesture bar's + and − buttons set it. */
export const areaDrawMode = new BehaviorSubject<'add' | 'subtract'>('add');
export const areaDrawMode$ = areaDrawMode.asObservable();

/** A stroke that encloses less than this share of the photo is treated as a mis-tap and ignored. */
const MIN_AREA_SHARE = 0.002;
const GIZMO_NAME = 'color-area';

/** Captures one freehand loop over the preview, shows it on the gizmos layer, and saves it for the color being drawn. */
export function ColorAreaOverlay() {
  const points = useRef<{ x: number; y: number }[]>([]);
  const gizmos = useObservable(editorGizmos$, editorGizmos.value);
  const brush = useMemo(() => {
    const density = PixelRatio.get();
    return new Brush()
      .withColor(Color.white())
      .withSize(Math.round(3 * density))
      .withHardness(1)
      .withOpacity(0.7) as Brush;
  }, []);
  useEffect(() => {
    previewCircling.next(true);
    return () => {
      previewCircling.next(false);
      gizmos?.remove(GIZMO_NAME);
      brush.uniffiDestroy();
    };
  }, [brush, gizmos]);

  const draw = (point: { x: number; y: number }) => {
    const density = PixelRatio.get();
    gizmos?.addPoint(GIZMO_NAME, point.x * density, point.y * density, brush);
  };
  const responder = useMemo(
    () =>
      PanResponder.create({
        onStartShouldSetPanResponder: () => true,
        onMoveShouldSetPanResponder: () => true,
        // A second finger belongs to the preview's pinch responder, not to the outline.
        onPanResponderTerminationRequest: () => true,
        onPanResponderGrant: event => {
          points.current = [screenPoint(event)];
          gizmos?.remove(GIZMO_NAME);
          draw(points.current[0]);
        },
        onPanResponderMove: event => {
          if (event.nativeEvent.touches.length > 1) return;
          const point = screenPoint(event);
          points.current.push(point);
          draw(point);
        },
        onPanResponderRelease: () => {
          saveArea(points.current);
          gizmos?.remove(GIZMO_NAME);
          points.current = [];
        },
        onPanResponderTerminate: () => {
          gizmos?.remove(GIZMO_NAME);
          points.current = [];
        },
      }),
    [gizmos, brush],
  );

  return <View style={StyleSheet.absoluteFill} {...responder.panHandlers} />;
}

/** The touch relative to the drawing layer, which covers the preview box, so it matches the box the photo is laid out in. */
function screenPoint(event: { nativeEvent: { locationX: number; locationY: number } }) {
  return { x: event.nativeEvent.locationX, y: event.nativeEvent.locationY };
}

/** Converts a screen loop to fractions of the photo and saves it for the active color. Strokes that are too small are ignored. */
function saveArea(screenPoints: readonly { x: number; y: number }[]) {
  const image = currentImage.value as AbraImageLike | null;
  const box = previewBox.value;
  if (!image || !box || screenPoints.length < 3) return;
  const width = image.width();
  const height = image.height();
  const polygon: { x: number; y: number }[] = [];
  for (const point of screenPoints) {
    const inImage = computePreviewImagePoint(
      width,
      height,
      box,
      previewScaleValue.value,
      previewTranslateValue.value,
      point,
      previewRotationValue.value,
    );
    if (inImage) polygon.push({ x: inImage.x / width, y: inImage.y / height });
  }
  if (polygon.length < 3) return;
  const share = Math.abs(
    polygon.reduce((sum, point, index) => {
      const next = polygon[(index + 1) % polygon.length];
      return sum + point.x * next.y - next.x * point.y;
    }, 0) / 2,
  );
  if (share < MIN_AREA_SHARE) return;
  const key = areaDrawKey.value;
  if (!key) return;
  const existing = areaPoints(adjustments.value[key]);
  const toPixels = (ring: number[][]) => ring.map(([x, y]) => ({ x: x * width, y: y * height }));
  const current = existing?.map(toPixels);
  const stroke = toPixels(polygon.map(point => [point.x, point.y]));
  // Subtracting from the whole photo starts from the full photo's rectangle.
  const whole = [{ x: 0, y: 0 }, { x: width, y: 0 }, { x: width, y: height }, { x: 0, y: height }];
  const rings =
    areaDrawMode.value === 'add'
      ? polygonUnion(current ?? [], [stroke])
      : polygonDifference(current ?? [whole], [stroke]);
  commitColorArea(
    key,
    encodeAreaRings(rings.map(ring => ring.map(point => [point.x / width, point.y / height]))),
  );
}

const AREA_OUTLINE_COLORS = ['#ffffff', '#ffd54f', '#4fc3f7'];

/**
 * The saved areas on screen. Areas show only while their control is open, and then only the color being edited.
 */
function visibleAreas(sectionKey: string, open: { key: string; index: number }, values: Record<string, AdjustmentValue>) {
  return leafControls(findSection(sectionKey).controls).flatMap(control => {
    if (control.kind !== 'slider' || !control.areaKeys || open.key !== control.key) return [];
    const key = control.areaKeys[open.index];
    const rings = key === undefined ? undefined : areaPoints(values[key]);
    return rings ? [{ index: open.index, rings }] : [];
  });
}

/** Draws the open color's saved area on the photo, in the same space as the person outlines so they zoom and turn with it. */
export function AreaOutlines({ imageSize, previewSize }: { imageSize: { width: number; height: number }; previewSize: PreviewBox }) {
  const sectionKey = useObservable(activeSectionKey$, activeSectionKey.value);
  const values = useObservable(adjustments$, adjustments.value);
  const open = useObservable(openSliderState$, openSliderState.value);
  const scale = useObservable(previewScaleSettled$, previewScaleValue.value);
  const hidden = useObservable(areaOutlinesHidden$, areaOutlinesHidden.value);
  const areas = visibleAreas(sectionKey, open, values);
  if (hidden || areas.length === 0) return null;

  const fit = Math.min(previewSize.width / imageSize.width, previewSize.height / imageSize.height);
  const offsetX = (previewSize.width - imageSize.width * fit) / 2;
  const offsetY = (previewSize.height - imageSize.height * fit) / 2;
  // Divided by the zoom so the lines stay the same thickness on screen.
  const strokeWidth = 2 / scale;

  return (
    <Svg style={StyleSheet.absoluteFill} pointerEvents="none">
      {areas.map(({ index, rings }) => {
        // Every ring is one closed subpath; the even-odd fill makes a ring inside another a hole.
        const d = rings
          .map(ring =>
            ring
              .map(
                ([x, y], i) => `${i === 0 ? 'M' : 'L'} ${offsetX + x * imageSize.width * fit} ${offsetY + y * imageSize.height * fit}`,
              )
              .join(' ') + ' Z',
          )
          .join(' ');
        const color = AREA_OUTLINE_COLORS[index % AREA_OUTLINE_COLORS.length];
        return (
          <Path
            key={index}
            d={d}
            stroke={color}
            strokeWidth={strokeWidth}
            fill={color}
            fillOpacity={0.12}
            fillRule="evenodd"
            strokeLinejoin="round"
          />
        );
      })}
    </Svg>
  );
}
