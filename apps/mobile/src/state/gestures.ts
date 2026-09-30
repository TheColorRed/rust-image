import { Animated, PanResponder, type GestureResponderEvent } from 'react-native';
import { BehaviorSubject, combineLatest, merge, Observable, Subject, type Subscription } from 'rxjs';
import { filter, map, startWith, tap } from 'rxjs/operators';
import { isBlemishToolFocused } from '@/src/state/edits';

// #region: Preview layout

/** On-screen size of the preview area; gesture bounds and blemish targeting are relative to it. */
export const previewBox = new BehaviorSubject<{ width: number; height: number } | null>(null);
export const previewBox$ = previewBox.asObservable();
/** Pending rAF handle debouncing `onLayout` into `previewBox`. */
export const previewLayoutFrame = new BehaviorSubject<number | null>(null);
export const previewLayoutFrame$ = previewLayoutFrame.asObservable();

// #endregion

// #region: Preview transform and gesture state

export const previewScale = new BehaviorSubject(new Animated.Value(1));
export const previewScale$ = previewScale.asObservable();
export const previewTranslateX = new BehaviorSubject(new Animated.Value(0));
export const previewTranslateX$ = previewTranslateX.asObservable();
export const previewTranslateY = new BehaviorSubject(new Animated.Value(0));
export const previewTranslateY$ = previewTranslateY.asObservable();
export const previewScaleValue = new BehaviorSubject(1);
export const previewScaleValue$ = previewScaleValue.asObservable();
export const previewTranslateValue = new BehaviorSubject({ x: 0, y: 0 });
export const previewTranslateValue$ = previewTranslateValue.asObservable();
export const gestureStart = new BehaviorSubject({
  active: false,
  touchCount: 1 as 1 | 2,
  distance: 0,
  midpoint: { x: 0, y: 0 },
  focalPoint: { x: 0, y: 0 },
  scale: 1,
  translate: { x: 0, y: 0 },
});
export const gestureStart$ = gestureStart.asObservable();
export const previewPanResponder = new BehaviorSubject<ReturnType<typeof PanResponder.create> | null>(null);
export const previewPanResponder$ = previewPanResponder.asObservable();

// #endregion

// #region: Preview gesture events

export const MIN_PREVIEW_ZOOM = 1;
export const MAX_PREVIEW_ZOOM = 100;
/** How far (screen px) a single finger must move before it's treated as a pan instead of a
 * potential long-press — only relevant while already zoomed in. */
export const PAN_START_THRESHOLD = 6;

type Touches = GestureResponderEvent['nativeEvent']['touches'];

/** Raw touch stream from the preview's PanResponder; `startPreviewGestures` turns it into transforms. */
export type PreviewTouchEvent = { type: 'grant' | 'move' | 'release' | 'terminate'; touches: Touches };
export const previewTouch = new Subject<PreviewTouchEvent>();

export const previewPressed = new Subject<GestureResponderEvent>();
export const previewPressed$ = previewPressed.asObservable();

// #endregion

// #region: Preview transform

/** Emits `initial` (the value it was created with), then every change of the Animated value. */
const animatedValue$ = (value: Animated.Value, initial: number) =>
  new Observable<number>(subscriber => {
    const id = value.addListener(({ value: next }) => subscriber.next(next));
    return () => value.removeListener(id);
  }).pipe(startWith(initial));

/**
 * Mirrors the Animated values into `previewScaleValue` / `previewTranslateValue` so gesture math
 * and tools can read them synchronously. Returns the subscription to tear down on unmount.
 */
export function trackPreviewTransform(): Subscription {
  const subscription = animatedValue$(previewScale.value, 1).subscribe(value => previewScaleValue.next(value));
  subscription.add(
    combineLatest([animatedValue$(previewTranslateX.value, 0), animatedValue$(previewTranslateY.value, 0)])
      .pipe(map(([x, y]) => ({ x, y })))
      .subscribe(translate => previewTranslateValue.next(translate)),
  );
  return subscription;
}

/** Puts the preview back at 1x, centered. */
export function resetPreviewTransform() {
  previewScale.value.setValue(1);
  previewTranslateX.value.setValue(0);
  previewTranslateY.value.setValue(0);
}

/** Springs the preview to `to` (and optionally `to.scale`) with the native driver. */
export function springPreview(to: { x: number; y: number; scale?: number }) {
  Animated.parallel([
    ...(to.scale === undefined ? [] : [Animated.spring(previewScale.value, { toValue: to.scale, useNativeDriver: true })]),
    Animated.spring(previewTranslateX.value, { toValue: to.x, useNativeDriver: true }),
    Animated.spring(previewTranslateY.value, { toValue: to.y, useNativeDriver: true }),
  ]).start();
}

// #endregion

// #region: Pinch and pan

type GestureBaseline = ReturnType<typeof gestureStart.getValue>;

const distanceBetweenTouches = (touches: Touches) => {
  const [a, b] = touches;
  return Math.hypot(a.pageX - b.pageX, a.pageY - b.pageY);
};

const midpointOfTouches = (touches: Touches) => {
  const [a, b] = touches;
  return { x: (a.pageX + b.pageX) / 2, y: (a.pageY + b.pageY) / 2 };
};

const localMidpointOfTouches = (touches: Touches) => {
  const [a, b] = touches;
  return { x: (a.locationX + b.locationX) / 2, y: (a.locationY + b.locationY) / 2 };
};

const clamp = (limit: number, value: number) => Math.min(limit, Math.max(-limit, value));

// Baselines the gesture from the touches present right now (on grant, or when a finger is lifted or
// added mid-gesture), so the next move computes a delta from here instead of jumping from wherever
// the previous touch count left off.
function baselineOf(touches: Touches): GestureBaseline {
  const two = touches.length === 2;
  return {
    active: true,
    touchCount: two ? 2 : 1,
    distance: two ? distanceBetweenTouches(touches) : 0,
    // Screen coordinates for movement: `locationX/Y` are measured in the scaled child view, so
    // their deltas shrink as the preview gets larger.
    midpoint: two ? midpointOfTouches(touches) : { x: touches[0].pageX, y: touches[0].pageY },
    // Local coordinates are only needed for the initial zoom anchor relative to the preview center.
    focalPoint: two ? localMidpointOfTouches(touches) : { x: touches[0].locationX, y: touches[0].locationY },
    scale: previewScaleValue.value,
    translate: previewTranslateValue.value,
  };
}

function transformFor(touches: Touches, start: GestureBaseline, box: { width: number; height: number }) {
  const scale =
    touches.length === 2
      ? Math.min(
          MAX_PREVIEW_ZOOM,
          Math.max(MIN_PREVIEW_ZOOM, start.scale * (distanceBetweenTouches(touches) / start.distance)),
        )
      : start.scale;
  const midpoint = touches.length === 2 ? midpointOfTouches(touches) : { x: touches[0].pageX, y: touches[0].pageY };
  // Scaling happens about the preview's center; counteract that so the image pixel under the
  // initial pinch midpoint stays under the fingers as their distance changes.
  const scaleRatio = scale / start.scale;
  const focalOffsetX = start.focalPoint.x - box.width / 2 - start.translate.x;
  const focalOffsetY = start.focalPoint.y - box.height / 2 - start.translate.y;
  // Permit overscroll far enough that either image edge can reach the centered blemish reticle.
  return {
    scale,
    x: clamp(
      (scale * box.width) / 2,
      start.translate.x + (midpoint.x - start.midpoint.x) + (1 - scaleRatio) * focalOffsetX,
    ),
    y: clamp(
      (scale * box.height) / 2,
      start.translate.y + (midpoint.y - start.midpoint.y) + (1 - scaleRatio) * focalOffsetY,
    ),
  };
}

type GestureStep =
  | { kind: 'baseline'; touches: Touches }
  | { kind: 'transform'; scale: number; x: number; y: number }
  | { kind: 'end' };

const isPinchOrDrag = (touches: Touches) => touches.length === 1 || touches.length === 2;

/** Turns the raw touch stream into gesture baselines, Animated transforms and the release snap-back. */
export function startPreviewGestures(): Subscription {
  const grants$ = previewTouch.pipe(
    filter(event => event.type === 'grant' && isPinchOrDrag(event.touches)),
    map((event): GestureStep => ({ kind: 'baseline', touches: event.touches })),
  );
  const moves$ = previewTouch.pipe(
    filter(event => event.type === 'move' && isPinchOrDrag(event.touches)),
    filter(() => gestureStart.value.active && !!previewBox.value),
    map((event): GestureStep => {
      const start = gestureStart.value;
      // A finger was lifted or added since the last move (e.g. a pinch settling down to one finger
      // still dragging) — re-baseline instead of jumping to a stale delta.
      if (event.touches.length !== start.touchCount) return { kind: 'baseline', touches: event.touches };
      return { kind: 'transform', ...transformFor(event.touches, start, previewBox.value!) };
    }),
  );
  const ends$ = previewTouch.pipe(
    filter(event => event.type === 'release' || event.type === 'terminate'),
    tap(event => {
      // Snap fully back to centered/unzoomed once the pinch closes back down near 1x, rather than
      // leaving it a hair off — a deliberate zoom-out should feel like it actually reset.
      if (event.type === 'release' && !isBlemishToolFocused() && previewScaleValue.value <= MIN_PREVIEW_ZOOM + 0.05) {
        springPreview({ x: 0, y: 0, scale: 1 });
      }
    }),
    map((): GestureStep => ({ kind: 'end' })),
  );

  return merge(grants$, moves$, ends$).subscribe(step => {
    if (step.kind === 'baseline') {
      gestureStart.next(baselineOf(step.touches));
    } else if (step.kind === 'end') {
      gestureStart.next({ ...gestureStart.value, active: false });
    } else {
      previewScale.value.setValue(step.scale);
      previewTranslateX.value.setValue(step.x);
      previewTranslateY.value.setValue(step.y);
    }
  });
}

/**
 * The PanResponder that feeds `previewTouch`. Capture (rather than bubble) handlers so a stationary
 * single finger still reaches the Pressable below untouched (long-press for the checkpoint preview
 * keeps working). A second finger always steals the responder immediately (pinch); a single finger
 * only once it has moved — while zoomed in, that's a pan, not a long-press. Built on PanResponder +
 * Animated rather than gesture-handler/reanimated, neither of which is otherwise a dependency.
 */
export function createPreviewPanResponder() {
  return PanResponder.create({
    onStartShouldSetPanResponderCapture: event => event.nativeEvent.touches.length === 2,
    onMoveShouldSetPanResponderCapture: (event, gestureState) => {
      if (event.nativeEvent.touches.length === 2) return true;
      const moved = Math.hypot(gestureState.dx, gestureState.dy) > PAN_START_THRESHOLD;
      // At 1x, normal editing leaves taps alone, but blemish work needs to pan an edge under its
      // fixed center reticle without making the user zoom first.
      if (isBlemishToolFocused()) return moved;
      if (previewScaleValue.value <= MIN_PREVIEW_ZOOM + 0.01) return false;
      return moved;
    },
    onPanResponderGrant: event => previewTouch.next({ type: 'grant', touches: event.nativeEvent.touches }),
    onPanResponderMove: event => previewTouch.next({ type: 'move', touches: event.nativeEvent.touches }),
    onPanResponderRelease: event => previewTouch.next({ type: 'release', touches: event.nativeEvent.touches }),
    onPanResponderTerminate: event => previewTouch.next({ type: 'terminate', touches: event.nativeEvent.touches }),
  });
}

export function ensurePreviewPanResponder() {
  if (!previewPanResponder.value) previewPanResponder.next(createPreviewPanResponder());
}

// #endregion
