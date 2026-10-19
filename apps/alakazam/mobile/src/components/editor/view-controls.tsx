import { useTheme } from '@/src/lib/ui/theme';
import {
  MAX_PREVIEW_ZOOM,
  MIN_PREVIEW_ZOOM,
  previewBox,
  previewCircling,
  previewControlsCover,
  previewOrigin,
  previewZoomFloor,
  previewScaleValue,
  previewTranslateValue,
  previewMoveMode,
  previewMoveMode$,
  currentPanOverride,
  previewPanOverride$,
  previewRotationGlide,
  previewRotationLocked,
  previewRotationLocked$,
  previewRotationValue,
  previewRotationValue$,
  previewScaleSettled$,
  springPreview,
} from '@/src/state/gestures';
import { Grab, Hand, Lock, Maximize, Minus, Pencil, Plus, Pointer, RotateCw, Undo2, ZoomIn } from 'lucide-react-native';
import { useEffect, useMemo, useRef, useState } from 'react';
import { PanResponder, PixelRatio, Pressable, ScrollView, StyleSheet, Text, View } from 'react-native';
import { Color } from '@abra/core/color';
import { Brush } from '@abra/core/drawing';
import { editorGizmos, editorGizmos$ } from '@/src/state/gizmos';
import { useObservable } from 'react-rx';
import { merge } from 'rxjs';
import { debounceTime, skip } from 'rxjs/operators';
import {
  activeSectionKey,
  activeSectionKey$,
  adjustments,
  adjustments$,
  focusedControlKey,
  focusedControlKey$,
  openSliderState,
  openSliderState$,
} from '@/src/state/edits';
import { areaDrawKey, areaDrawKey$, areaDrawMode, areaDrawMode$ } from '@/src/components/tools/color-area-tool';
import { areaPoints, findSection, leafControls, sliderSpecs, sliderValues } from '@/src/lib/project/edit-sections';

const ICON = '#fff';

/** The photo's turn once it stops changing, so a twist does not re-render this every frame. */
/** The Move pill steps through these in order: any fingers, one finger only, two fingers only, then locked. */
const MOVE_MODES = {
  move: { label: 'Pan: any', icon: Hand, next: 'one-finger' },
  'one-finger': { label: 'Pan: one finger', icon: Pointer, next: 'two-finger' },
  'two-finger': { label: 'Pan: two fingers', icon: Grab, next: 'off' },
  off: { label: 'Pan: locked', icon: Lock, next: 'move' },
} as const;

const settledTurn$ = previewRotationValue$.pipe(debounceTime(120));

/**
 * Floats over the bottom corner of the photo. "Turn photo" is always there and off until the person asks for it, so a pinch only
 * zooms; when it is on, two fingers also twist the photo, and a hint says so. "Reset zoom" and "Straighten" show only while there is
 * something to put back.
 */
export function ViewControls() {
  const { colors } = useTheme();
  const styles = useMemo(() => createStyles(colors), [colors]);
  const zoom = useObservable(previewScaleSettled$, 1);
  const turn = useObservable(settledTurn$, previewRotationValue.value);
  const locked = useObservable(previewRotationLocked$, previewRotationLocked.value);
  const moveMode = useObservable(previewMoveMode$, previewMoveMode.value);
  const panOverride = useObservable(previewPanOverride$, currentPanOverride());
  const move = panOverride ? { label: `Pan: locked by ${panOverride.tool}`, icon: Lock } : MOVE_MODES[moveMode];
  const MoveIcon = move.icon;
  const [circling, setCircling] = useState(false);
  const sectionKey = useObservable(activeSectionKey$, activeSectionKey.value);
  const edits = useObservable(adjustments$, adjustments.value);
  const focusedKey = useObservable(focusedControlKey$, focusedControlKey.value);
  const openSlider = useObservable(openSliderState$, openSliderState.value);
  const drawing = useObservable(areaDrawKey$, areaDrawKey.value);
  const drawMode = useObservable(areaDrawMode$, areaDrawMode.value);
  // A tool that paints an area (today only Color Splash) gets the Draw area button. It targets the slider tab that is open.
  const focusedSlider = focusedKey ? leafControls(findSection(sectionKey).controls).find(control => control.key === focusedKey) : undefined;
  const drawable = focusedSlider?.kind === 'slider' && focusedSlider.areaKeys ? focusedSlider : null;
  const drawIndex = drawable && openSlider.key === drawable.key ? openSlider.index : 0;
  const drawKey = drawable?.areaKeys?.[drawIndex] ?? null;
  // A color must be on before an area can attach to it.
  const drawValue = drawable ? sliderValues(drawable, edits[drawable.key])[drawIndex] : 0;
  const canDraw = drawKey !== null && drawValue > (drawable ? sliderSpecs(drawable)[drawIndex]?.min ?? 0 : 0);
  // "+" and "−" only edit an area that is already drawn. "Draw area" starts the first one.
  const hasArea = drawKey !== null && (areaPoints(edits[drawKey]) ?? []).length > 0;
  const drawActive = drawKey !== null && drawing === drawKey;
  // "+" draws to add to the color's area and "−" draws to take away from it. Tapping the one that is on turns drawing off.
  const toggleDraw = (mode: 'add' | 'subtract') => {
    if (drawActive && areaDrawMode.value === mode) {
      areaDrawKey.next(null);
      return;
    }
    areaDrawMode.next(mode);
    areaDrawKey.next(drawKey);
  };
  const zoomed = zoom > 1.01;
  const turned = Math.abs(turn) > 0.01;

  useEffect(() => {
    // Changing the tool or the open color ends the drawing, so an area never lands on the wrong color.
    const subscription = merge(focusedControlKey$, openSliderState$)
      .pipe(skip(1))
      .subscribe(() => areaDrawKey.next(null));
    return () => subscription.unsubscribe();
  }, []);

  const wholeTurns = () => Math.round(previewRotationValue.value / (2 * Math.PI)) * 2 * Math.PI;
  return (
    <>
    {circling ? <CircleToZoomOverlay onDone={() => setCircling(false)} /> : null}
    <View
      style={styles.area}
      pointerEvents="box-none"
      // The row sits 12 px above the preview's bottom edge (see `area`), so the cover includes that gap.
      onLayout={event => previewControlsCover.next(event.nativeEvent.layout.height + 12)}
    >
      {circling ? <Text style={styles.hint}>Draw a circle around what to zoom into</Text> : null}
      {!circling && drawActive ? (
        <Text style={styles.hint}>{drawMode === 'add' ? 'Draw to add to this color' : 'Draw to take away from this color'}</Text>
      ) : null}
      {!circling && !locked && !turned ? <Text style={styles.hint}>Twist with two fingers to turn the photo</Text> : null}
      <View style={styles.barRow}>
      <ScrollView horizontal showsHorizontalScrollIndicator={false} style={styles.rowScroll} contentContainerStyle={styles.row}>
        <View style={styles.group}>
          <Pressable
            accessibilityRole="switch"
            accessibilityState={{ checked: circling }}
            accessibilityLabel="Circle to zoom"
            style={[styles.segment, circling && styles.pillOn]}
            hitSlop={8}
            onPress={() => setCircling(!circling)}
          >
            <ZoomIn color={ICON} size={16} />
            <Text style={styles.label}>Circle to zoom</Text>
          </Pressable>
          <View style={styles.divider} />
          <Pressable
            accessibilityRole="button"
            accessibilityLabel="Zoom out"
            style={styles.segment}
            hitSlop={8}
            onPress={() => {
              const floor = previewZoomFloor.value;
              const next = Math.max(floor, previewScaleValue.value / ZOOM_STEP);
              const ratio = next / previewScaleValue.value;
              springPreview(
                next <= floor && floor >= MIN_PREVIEW_ZOOM
                  ? { x: 0, y: 0, scale: 1 }
                  : { x: previewTranslateValue.value.x * ratio, y: previewTranslateValue.value.y * ratio, scale: next },
              );
            }}
          >
            <Minus color={ICON} size={16} />
          </Pressable>
          <View style={styles.divider} />
          <Pressable
            accessibilityRole="button"
            accessibilityLabel="Zoom in"
            style={styles.segment}
            hitSlop={8}
            onPress={() => {
              const next = Math.min(MAX_PREVIEW_ZOOM, previewScaleValue.value * ZOOM_STEP);
              const ratio = next / previewScaleValue.value;
              springPreview({ x: previewTranslateValue.value.x * ratio, y: previewTranslateValue.value.y * ratio, scale: next });
            }}
          >
            <Plus color={ICON} size={16} />
          </Pressable>
        </View>
        <Pressable
          accessibilityRole="switch"
          accessibilityState={{ checked: !locked }}
          accessibilityLabel="Turnable"
          style={[styles.pill, !locked && styles.pillOn]}
          hitSlop={8}
          onPress={() => {
            // Turning off a photo that is turned brings it upright first, so it is not left crooked with no way to twist it back.
            if (!locked) previewRotationGlide.next(wholeTurns());
            previewRotationLocked.next(!locked);
          }}
        >
          <RotateCw color={ICON} size={16} />
          <Text style={styles.label}>Turnable</Text>
        </Pressable>
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={
            panOverride ? `Move: locked by ${panOverride.tool}` : `Move: ${MOVE_MODES[moveMode].label}. Tap to change`
          }
          accessibilityState={{ disabled: !!panOverride }}
          disabled={!!panOverride}
          style={[styles.pill, panOverride && styles.pillLocked]}
          hitSlop={8}
          onPress={() => previewMoveMode.next(MOVE_MODES[moveMode].next)}
        >
          <MoveIcon color={ICON} size={16} />
          <Text style={styles.label}>{move.label}</Text>
        </Pressable>
        {turned ? (
          <Pressable style={styles.pill} hitSlop={8} onPress={() => previewRotationGlide.next(wholeTurns())}>
            <Undo2 color={ICON} size={16} />
            <Text style={styles.label}>Straighten</Text>
          </Pressable>
        ) : null}
        {zoomed ? (
          <Pressable style={styles.pill} hitSlop={8} onPress={() => springPreview({ x: 0, y: 0, scale: 1 })}>
            <Maximize color={ICON} size={16} />
            <Text style={styles.label}>Reset zoom</Text>
          </Pressable>
        ) : null}
      </ScrollView>
      {drawable ? (
        <View style={styles.group}>
          <Pressable
            accessibilityRole="switch"
            accessibilityState={{ checked: drawActive, disabled: !canDraw }}
            accessibilityLabel="Draw area"
            disabled={!canDraw}
            style={[styles.segment, drawActive && styles.pillOn, !canDraw && styles.pillLocked]}
            hitSlop={8}
            onPress={() => toggleDraw('add')}
          >
            <Pencil color={ICON} size={16} />
            <Text style={styles.label}>Draw area</Text>
          </Pressable>
          <View style={styles.divider} />
          <Pressable
            accessibilityRole="button"
            accessibilityLabel="Draw to add to this color"
            accessibilityState={{ checked: drawActive && drawMode === 'add', disabled: !canDraw || !hasArea }}
            disabled={!canDraw || !hasArea}
            style={[styles.segment, drawActive && drawMode === 'add' && styles.pillOn, (!canDraw || !hasArea) && styles.pillLocked]}
            hitSlop={8}
            onPress={() => toggleDraw('add')}
          >
            <Plus color={ICON} size={16} />
          </Pressable>
          <View style={styles.divider} />
          <Pressable
            accessibilityRole="button"
            accessibilityLabel="Draw to take away from this color"
            accessibilityState={{ checked: drawActive && drawMode === 'subtract', disabled: !canDraw || !hasArea }}
            disabled={!canDraw || !hasArea}
            style={[styles.segment, drawActive && drawMode === 'subtract' && styles.pillOn, (!canDraw || !hasArea) && styles.pillLocked]}
            hitSlop={8}
            onPress={() => toggleDraw('subtract')}
          >
            <Minus color={ICON} size={16} />
          </Pressable>
        </View>
      ) : null}
      </View>
    </View>
    </>
  );
}

/** How much one tap of − or + zooms. */
const ZOOM_STEP = 1.5;
/** How much of the box the circled area fills once zoomed, so the circle's edge is not touching the sides. */
const CIRCLE_FILL = 0.9;
/** A circle smaller than this (screen px across) is taken as a stray touch. */
const MIN_CIRCLE = 24;

/**
 * Covers the photo while "Circle to zoom" is on. One finger draws a loop; when it lifts, the photo glides so the area inside the
 * loop's bounds fills the box, and the mode turns itself off.
 */
function CircleToZoomOverlay({ onDone }: { onDone: () => void }) {
  const points = useRef<{ x: number; y: number }[]>([]);
  // The library draws the loop: a soft glow under a crisp line, on the gizmos view over the photo. Its sizes are in the view's pixels.
  const gizmos = useObservable(editorGizmos$, editorGizmos.value);
  const { glow, line } = useMemo(() => {
    const density = PixelRatio.get();
    return {
      glow: new Brush().withColor(Color.white()).withSize(Math.round(5 * density)).withHardness(0).withOpacity(0.5) as Brush,
      line: new Brush().withColor(Color.white()).withSize(Math.round(1 * density)).withHardness(1) as Brush,
    };
  }, []);
  // The loop goes away with the mode, and the brushes are let go of.
  useEffect(
    () => () => {
      gizmos?.remove('glow');
      gizmos?.remove('line');
      glow.uniffiDestroy();
      line.uniffiDestroy();
    },
    [gizmos, glow, line],
  );
  const draw = (x: number, y: number) => {
    const density = PixelRatio.get();
    gizmos?.addPoint('glow', x * density, y * density, glow);
    gizmos?.addPoint('line', x * density, y * density, line);
  };
  const finish = useRef(onDone);
  finish.current = onDone;
  useEffect(() => {
    previewCircling.next(true);
    return () => previewCircling.next(false);
  }, []);

  const responder = useMemo(
    () =>
      PanResponder.create({
        onStartShouldSetPanResponder: () => true,
        onPanResponderTerminationRequest: () => false,
        onMoveShouldSetPanResponder: () => true,
        onPanResponderGrant: event => {
          gizmos?.remove('glow');
          gizmos?.remove('line');
          const point = { x: event.nativeEvent.pageX - previewOrigin.value.x, y: event.nativeEvent.pageY - previewOrigin.value.y };
          points.current = [point];
          draw(point.x, point.y);
        },
        onPanResponderMove: event => {
          const point = { x: event.nativeEvent.pageX - previewOrigin.value.x, y: event.nativeEvent.pageY - previewOrigin.value.y };
          points.current.push(point);
          draw(point.x, point.y);
        },
        onPanResponderRelease: () => {
          const box = previewBox.value;
          const drawn = points.current;
          if (!box || drawn.length < 2) return finish.current();
          const scale = previewScaleValue.value;
          const rotation = previewRotationValue.value;
          const translate = previewTranslateValue.value;
          const cos = Math.cos(rotation);
          const sin = Math.sin(rotation);
          // Each point on the screen, back to where it is on the unzoomed, unturned photo (offset from the middle of the box).
          const onPhoto = drawn.map(point => {
            const x = point.x - box.width / 2;
            const y = point.y - box.height / 2;
            return { x: (cos * x + sin * y - translate.x) / scale, y: (-sin * x + cos * y - translate.y) / scale };
          });
          const left = Math.min(...onPhoto.map(point => point.x));
          const right = Math.max(...onPhoto.map(point => point.x));
          const top = Math.min(...onPhoto.map(point => point.y));
          const bottom = Math.max(...onPhoto.map(point => point.y));
          const across = Math.max(right - left, bottom - top);
          if (across * scale >= MIN_CIRCLE) {
            const next = Math.min(
              MAX_PREVIEW_ZOOM,
              Math.max(MIN_PREVIEW_ZOOM, Math.min((box.width * CIRCLE_FILL) / (right - left), (box.height * CIRCLE_FILL) / (bottom - top))),
            );
            springPreview({ x: (-(left + right) / 2) * next, y: (-(top + bottom) / 2) * next, scale: next });
          }
          finish.current();
        },
        onPanResponderTerminate: () => finish.current(),
      }),
    [gizmos, glow, line],
  );

  return (
    <View style={StyleSheet.absoluteFill} {...responder.panHandlers} />
  );
}

function createStyles(colors: ReturnType<typeof useTheme>['colors']) {
  return StyleSheet.create({
    area: { position: 'absolute', left: 12, right: 12, bottom: 12, gap: 8, alignItems: 'flex-start' },
    rowScroll: { flex: 1, minWidth: 0 },
    barRow: { flexDirection: 'row', alignItems: 'center', alignSelf: 'stretch', gap: 8 },
    row: { flexDirection: 'row', gap: 8 },
    pill: {
      flexDirection: 'row',
      alignItems: 'center',
      gap: 6,
      height: 34,
      paddingHorizontal: 12,
      borderRadius: 17,
      backgroundColor: colors.overlayButton,
    },
    group: { flexDirection: 'row', alignItems: 'center', height: 34, borderRadius: 17, overflow: 'hidden', backgroundColor: colors.overlayButton },
    segment: { flexDirection: 'row', alignItems: 'center', justifyContent: 'center', gap: 6, height: 34, paddingHorizontal: 12 },
    divider: { width: 1, alignSelf: 'stretch', backgroundColor: 'rgba(255,255,255,0.35)' },
    pillOn: { backgroundColor: colors.accent },
    pillLocked: { opacity: 0.6 },
    label: { color: ICON, fontSize: 13, fontWeight: '600' },
    hint: {
      color: ICON,
      fontSize: 12,
      paddingHorizontal: 10,
      paddingVertical: 4,
      borderRadius: 10,
      overflow: 'hidden',
      backgroundColor: colors.overlayButton,
    },
  });
}
