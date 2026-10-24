import { devDebug } from '@/src/lib/log';
import { Animated, PanResponder, type GestureResponderEvent } from 'react-native';
import { BehaviorSubject, combineLatest, merge, Observable, Subject, type Subscription } from 'rxjs';
import { debounceTime, distinctUntilChanged, filter, map, startWith, tap } from 'rxjs/operators';
import { BLEMISH_TOOL_KEY, CROP_TOOL_KEY } from '@/src/lib/project/edit-sections/keys';
import { focusedControlKey, focusedControlKey$, isBlemishToolFocused, isCropToolFocused } from '@/src/state/edits';
import { cropHandleGrabbed } from '@/src/state/crop';

// #region: Preview layout

/** On-screen size of the preview area; gesture bounds and blemish targeting are relative to it. */
export type PreviewBox = { width: number; height: number };
export const previewBox = new BehaviorSubject<PreviewBox | null>(null);
export const previewBox$ = previewBox.asObservable();
/** Pending rAF handle debouncing `onLayout` into `previewBox`. */
export const previewLayoutFrame = new BehaviorSubject<number | null>(null);
export const previewLayoutFrame$ = previewLayoutFrame.asObservable();
/** Window position of the preview's top-left, so pinch anchors don't depend on which child got the touch. */
export const previewOrigin = new BehaviorSubject({ x: 0, y: 0 });
/** How far the controls along the preview's bottom edge reach up from it, so a tool can keep clear of them. */
export const previewControlsCover = new BehaviorSubject(0);
export const previewControlsCover$ = previewControlsCover.asObservable();

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
/** The zoom once it stops changing, for overlays that would stutter the pinch if they re-rendered every frame. */
export const previewScaleSettled$ = previewScaleValue.pipe(debounceTime(120));
/** How far two fingers have turned the photo, clockwise, in radians. The overlay turns with the Animated value and the native views take the number. */
export const previewRotation = new BehaviorSubject(new Animated.Value(0));
export const previewRotation$ = previewRotation.asObservable();
export const previewRotationValue = new BehaviorSubject(0);
export const previewRotationValue$ = previewRotationValue.asObservable();
/** Whether two fingers are kept from turning the photo: a pinch only zooms and pans. On until the person unlocks it. */
export const previewRotationLocked = new BehaviorSubject(true);
export const previewRotationLocked$ = previewRotationLocked.asObservable();
/**
 * Which fingers slide the photo: 'move' is any number, 'one-finger' is a single finger (two fingers only zoom and turn about the
 * middle), and 'two-finger' is a pinch only (a single finger is left alone, e.g. for drawing).
 */
export type PreviewMoveMode = 'move' | 'one-finger' | 'two-finger' | 'off';
export const previewMoveMode = new BehaviorSubject<PreviewMoveMode>('move');
export const previewMoveMode$ = previewMoveMode.asObservable();
/**
 * A tool that needs its own panning. While the tool is open the pan is this mode, and the pan button is locked showing the tool's name.
 * The person's own setting in `previewMoveMode` is left as it is, so it returns when the tool closes.
 */
export type PanOverride = { mode: PreviewMoveMode; tool: string };
const TOOL_PAN_OVERRIDES: Record<string, PanOverride | undefined> = {
  // Any finger pans the photo while crop is open, except one that grabbed a corner handle (see createPreviewPanResponder).
  [CROP_TOOL_KEY]: { mode: 'move', tool: 'Crop' },
  // The blemish reticle needs the photo to slide under it at any zoom.
  [BLEMISH_TOOL_KEY]: { mode: 'move', tool: 'Blemish' },
};
const panOverrideFor = (key: string | null): PanOverride | null => (key && TOOL_PAN_OVERRIDES[key]) || null;
/** The pan a focused tool sets, or `null` when the person's own setting applies. */
export const currentPanOverride = (): PanOverride | null => panOverrideFor(focusedControlKey.value);
const samePanOverride = (a: PanOverride | null, b: PanOverride | null) =>
  a === b || (!!a && !!b && a.mode === b.mode && a.tool === b.tool);
export const previewPanOverride$ = focusedControlKey$.pipe(map(panOverrideFor), distinctUntilChanged(samePanOverride));
/** Whether the pan is held back for `fingers` touches, given the tool's pan or else the person's own. */
const panIsHeld = (fingers: number) => {
  const mode = currentPanOverride()?.mode ?? previewMoveMode.value;
  return mode === 'off' || (mode === 'one-finger' && fingers > 1) || (mode === 'two-finger' && fingers < 2);
};
/** Whether "Circle to zoom" is drawing. The preview's pan and pinch leave every touch alone meanwhile, so the loop is not taken as a pan. */
export const previewCircling = new BehaviorSubject(false);
/** Whether an area is being drawn on the photo (see `DrawAreaOverlay`). One finger draws, so it does not pan; two fingers still pinch and pan. */
export const previewDrawing = new BehaviorSubject(false);
/** Maps a zoom and pan to the ones the preview should use, e.g. to keep a crop frame covered by the photo. `null` leaves them alone. */
export type PreviewLimit = (scale: number, translate: { x: number; y: number }) => { scale: number; x: number; y: number };
export const previewLimit = new BehaviorSubject<PreviewLimit | null>(null);
export const previewTranslateValue = new BehaviorSubject({ x: 0, y: 0 });
export const previewTranslateValue$ = previewTranslateValue.asObservable();
export const gestureStart = new BehaviorSubject({
  active: false,
  touchCount: 1 as 1 | 2,
  distance: 0,
  /** The angle of the line between two fingers, and the photo's turn when they went down. */
  angle: 0,
  rotation: 0,
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
/** The lowest zoom the preview may reach. The crop tool lowers it so the whole photo can sit above the gestures row. */
export const previewZoomFloor = new BehaviorSubject(MIN_PREVIEW_ZOOM);
export const MAX_PREVIEW_ZOOM = 100;
/** How far (screen px) a single finger must move before it's treated as a pan instead of a
 * potential long-press — only relevant while already zoomed in. */
export const PAN_START_THRESHOLD = 6;

type Touches = GestureResponderEvent['nativeEvent']['touches'];

/** Raw touch stream from the preview's PanResponder; `startPreviewGestures` turns it into transforms. */
/** `velocity` (screen px per ms) is how fast the fingers were moving when they lifted. */
export type PreviewTouchEvent = {
  type: 'grant' | 'move' | 'release' | 'terminate';
  touches: Touches;
  velocity?: { x: number; y: number };
};
export const previewTouch = new Subject<PreviewTouchEvent>();

/** A tap relative to the untransformed preview, independent of which image child received it. */
export type PreviewPress = { x: number; y: number; pageX: number; pageY: number };
export const previewPressed = new Subject<PreviewPress>();
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
  subscription.add(
    new Observable<number>(subscriber => {
      const id = previewRotation.value.addListener(({ value }) => subscriber.next(value));
      return () => previewRotation.value.removeListener(id);
    }).subscribe(value => previewRotationValue.next(value)),
  );
  return subscription;
}

/** Puts the preview back at 1x, centered and upright. */
export function resetPreviewTransform() {
  previewRotation.value.setValue(0);
  previewScale.value.setValue(1);
  previewTranslateX.value.setValue(0);
  previewTranslateY.value.setValue(0);
}

/** How long the preview takes to glide to a new place. */
export const PREVIEW_GLIDE_MS = 380;

/**
 * A move of the preview for the editor preview to carry out, or `null` to stop the one under way. The preview owns the photo
 * views, so it hands the move to Vessel's tween (which makes a frame for every engine tick) and moves the outlines along the
 * same curve.
 */
export const previewGlide = new Subject<{ x: number; y: number; scale?: number } | null>();

/** Whether the preview is gliding to a new place. Heavy work on the JavaScript thread waits for this to be `false`, so it does not freeze the move. */
export const previewMoving = new BehaviorSubject(false);
export const previewMoving$ = previewMoving.asObservable();

/** A turn of the photo for the editor preview to carry out: the angle to glide to, in radians. Vessel's tween turns the views. */
export const previewRotationGlide = new Subject<number>();

/** Glides the preview to `to` (and optionally `to.scale`); with no preview shown it just goes there. */
export function springPreview(to: { x: number; y: number; scale?: number }) {
  devDebug('zoom', 'springPreview', JSON.stringify(to), 'observed:', previewGlide.observed);
  if (previewGlide.observed) return previewGlide.next(to);
  if (to.scale !== undefined) previewScale.value.setValue(to.scale);
  previewTranslateX.value.setValue(to.x);
  previewTranslateY.value.setValue(to.y);
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

const clamp = (limit: number, value: number) => Math.min(limit, Math.max(-limit, value));

// Baselines the gesture from the touches present right now (on grant, or when a finger is lifted or
// added mid-gesture), so the next move computes a delta from here instead of jumping from wherever
// the previous touch count left off.
function baselineOf(touches: Touches): GestureBaseline {
  const two = touches.length === 2;
  const midpoint = two ? midpointOfTouches(touches) : { x: touches[0].pageX, y: touches[0].pageY };
  return {
    active: true,
    touchCount: two ? 2 : 1,
    distance: two ? distanceBetweenTouches(touches) : 0,
    angle: two ? Math.atan2(touches[1].pageY - touches[0].pageY, touches[1].pageX - touches[0].pageX) : 0,
    rotation: previewRotationValue.value,
    // Screen coordinates for movement: `locationX/Y` are measured in the scaled child view, so
    // their deltas shrink as the preview gets larger.
    midpoint,
    // Zoom anchor relative to the preview's top-left. `locationX/Y` is relative to whichever child
    // was hit (e.g. the narrower photo view of a portrait image), which shifted the zoom sideways.
    focalPoint: { x: midpoint.x - previewOrigin.value.x, y: midpoint.y - previewOrigin.value.y },
    scale: previewScaleValue.value,
    translate: previewTranslateValue.value,
  };
}

function transformFor(touches: Touches, start: GestureBaseline, box: { width: number; height: number }) {
  const scale =
    touches.length === 2
      ? Math.min(
          MAX_PREVIEW_ZOOM,
          Math.max(previewZoomFloor.value, start.scale * (distanceBetweenTouches(touches) / start.distance)),
        )
      : start.scale;
  const midpoint = touches.length === 2 ? midpointOfTouches(touches) : { x: touches[0].pageX, y: touches[0].pageY };
  // Scaling happens about the preview's center; counteract that so the image pixel under the
  // initial pinch midpoint stays under the fingers as their distance changes.
  const scaleRatio = scale / start.scale;
  // The photo is turned on the screen, outside its pan, so the fingers' screen positions are turned back into its own axes.
  const cos = Math.cos(start.rotation);
  const sin = Math.sin(start.rotation);
  const alongPhoto = (x: number, y: number) => ({ x: cos * x + sin * y, y: -sin * x + cos * y });
  const focal = alongPhoto(start.focalPoint.x - box.width / 2, start.focalPoint.y - box.height / 2);
  const moved = alongPhoto(midpoint.x - start.midpoint.x, midpoint.y - start.midpoint.y);
  const focalOffsetX = focal.x - start.translate.x;
  const focalOffsetY = focal.y - start.translate.y;
  // Permit overscroll far enough that either image edge can reach the centered blemish reticle.
  // Two fingers turn the photo by how far the line between them has turned, kept within half a turn either way of where they started.
  const turned =
    touches.length === 2 && !previewRotationLocked.value
      ? Math.atan2(touches[1].pageY - touches[0].pageY, touches[1].pageX - touches[0].pageX) - start.angle
      : 0;
  const rotation = start.rotation + Math.atan2(Math.sin(turned), Math.cos(turned));
  // With the pan held, the zoom is about the middle of the box: the pan grows and shrinks with it, so the same part stays centered.
  if (panIsHeld(touches.length)) return { scale, rotation, x: start.translate.x * scaleRatio, y: start.translate.y * scaleRatio };
  return {
    scale,
    rotation,
    x: clamp(
      (scale * box.width) / 2,
      start.translate.x + moved.x + (1 - scaleRatio) * focalOffsetX,
    ),
    y: clamp(
      (scale * box.height) / 2,
      start.translate.y + moved.y + (1 - scaleRatio) * focalOffsetY,
    ),
  };
}

type GestureStep =
  | { kind: 'baseline'; touches: Touches }
  | { kind: 'transform'; scale: number; rotation: number; x: number; y: number }
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
      const next = transformFor(event.touches, start, previewBox.value!);
      // Limits apply to this gesture's result, not to the current value, so they never build up across moves.
      const limit = previewLimit.value;
      if (!limit) return { kind: 'transform', ...next };
      return { kind: 'transform', rotation: next.rotation, ...limit(next.scale, { x: next.x, y: next.y }) };
    }),
  );
  const ends$ = previewTouch.pipe(
    filter(event => event.type === 'release' || event.type === 'terminate'),
    tap(event => {
      // Snap fully back to centered/unzoomed once the pinch closes back down near 1x, rather than
      // leaving it a hair off — a deliberate zoom-out should feel like it actually reset.
      // A limit (the crop frame) owns the zoom and pan, so the snap-back and the coast would both move the photo off it.
      if (previewLimit.value) return;
      if (event.type === 'release' && !isBlemishToolFocused() && previewScaleValue.value <= MIN_PREVIEW_ZOOM + 0.05) {
        springPreview({ x: 0, y: 0, scale: 1 });
      } else if (event.type === 'release' && event.velocity && previewScaleValue.value > MIN_PREVIEW_ZOOM && !panIsHeld(gestureStart.value.touchCount)) {
        // Keep gliding the way the fingers were going, slowing down like a game camera coasting to a stop.
        Animated.parallel([
          Animated.decay(previewTranslateX.value, { velocity: event.velocity.x, deceleration: 0.995, useNativeDriver: true }),
          Animated.decay(previewTranslateY.value, { velocity: event.velocity.y, deceleration: 0.995, useNativeDriver: true }),
        ]).start();
      }
    }),
    map((): GestureStep => ({ kind: 'end' })),
  );

  return merge(grants$, moves$, ends$).subscribe(step => {
    if (step.kind === 'baseline') {
      // A new touch catches the glide instead of fighting it.
      previewTranslateX.value.stopAnimation();
      previewTranslateY.value.stopAnimation();
      gestureStart.next(baselineOf(step.touches));
    } else if (step.kind === 'end') {
      gestureStart.next({ ...gestureStart.value, active: false });
    } else {
      previewScale.value.setValue(step.scale);
      previewRotation.value.setValue(step.rotation);
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
    onStartShouldSetPanResponderCapture: event => !previewCircling.value && event.nativeEvent.touches.length === 2,
    onMoveShouldSetPanResponderCapture: (event, gestureState) => {
      if (previewCircling.value) return false;
      if (event.nativeEvent.touches.length === 2) return true;
      if (previewDrawing.value) return false;
      const moved = Math.hypot(gestureState.dx, gestureState.dy) > PAN_START_THRESHOLD;
      // While crop is open, a finger on a corner handle belongs to that handle; any other finger moves the photo under the frame.
      if (isCropToolFocused()) return moved && !cropHandleGrabbed.value;
      if (panIsHeld(1)) return false;
      // At 1x, normal editing leaves taps alone, but blemish work needs to pan an edge under its
      // fixed center reticle without making the user zoom first.
      if (isBlemishToolFocused()) return moved;
      if (previewScaleValue.value <= MIN_PREVIEW_ZOOM + 0.01) return false;
      return moved;
    },
    onPanResponderGrant: event => previewTouch.next({ type: 'grant', touches: event.nativeEvent.touches }),
    onPanResponderMove: event => previewTouch.next({ type: 'move', touches: event.nativeEvent.touches }),
    onPanResponderRelease: (event, gestureState) =>
      previewTouch.next({
        type: 'release',
        touches: event.nativeEvent.touches,
        velocity: { x: gestureState.vx, y: gestureState.vy },
      }),
    onPanResponderTerminate: event => previewTouch.next({ type: 'terminate', touches: event.nativeEvent.touches }),
  });
}

export function ensurePreviewPanResponder() {
  if (!previewPanResponder.value) previewPanResponder.next(createPreviewPanResponder());
}

// #endregion
