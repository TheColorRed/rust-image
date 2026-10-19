import { ChevronLeft } from 'lucide-react-native';
import { Fragment, useEffect, useMemo, useRef } from 'react';
import { useObservable } from 'react-rx';
import { combineLatest } from 'rxjs';
import {
  PanResponder,
  Pressable,
  ScrollView,
  StyleSheet,
  Text,
  View,
  type GestureResponderHandlers,
  type ViewStyle,
} from 'react-native';
import { useTheme, type ThemeColors } from '@/src/lib/ui/theme';
import {
  CROP_ASPECTS,
  FULL_CROP,
  aspectRatio,
  coverCropFrame,
  cropToScreen,
  cropZoomFloor,
  dpPerPhotoPixel,
  fitCropInTurn,
  fitCropToAspect,
  isFullCrop,
  moveCropCorner,
  panToCentreAboveBar,
  screenToCrop,
  turnedPhoto,
  zoomToFrame,
  photoBoundsOnScreen,
  slideFrameInto,
  zoomOutToFrame,
  panToKeepPoint,
  type CropAspect,
  type CropCorner,
  type CropRect,
  type Photo,
  type ScreenRect,
} from '@/src/lib/tools/crop';
import { commitCrop } from '@/src/lib/editor/editor-history';
import { flushFullReplay } from '@/src/lib/editor/editor-replay';
import { cropApplyRequested$ } from '@/src/state/commands';
import { cropAspect, cropAspect$, cropFrame, cropFrame$, cropHandleGrabbed } from '@/src/state/crop';
import { focusedControlKey, isCropToolFocused$ } from '@/src/state/edits';
import {
  MAX_PREVIEW_ZOOM,
  MIN_PREVIEW_ZOOM,
  previewBox,
  previewBox$,
  previewControlsCover,
  previewControlsCover$,
  previewLimit,
  previewMoving$,
  previewRotation,
  previewRotationGlide,
  previewRotationValue,
  previewRotationValue$,
  previewScale,
  previewScaleValue,
  previewScaleValue$,
  previewZoomFloor,
  previewTranslateValue,
  previewTranslateValue$,
  previewTranslateX,
  previewTranslateY,
  springPreview,
  type PreviewBox,
} from '@/src/state/gestures';
import { historyReady } from '@/src/state/project-session';
import { currentImage } from '@/src/state/session';

/** Size (dp) of the square a corner handle can be grabbed in, and of the bracket drawn inside it. */
const HANDLE_HIT = 44;
const BRACKET = 24;
const BRACKET_LINE = 4;
const CORNERS: CropCorner[] = ['top-left', 'top-right', 'bottom-right', 'bottom-left'];
/** Radians below which a turn counts as none. */
const TURN_EPSILON = 0.001;

/** The photo's size as the crop reads it, or `null` while there is no photo. */
function photoSize(): Photo | null {
  const image = currentImage.value;
  if (!image || image.width() === 0 || image.height() === 0) return null;
  return { width: image.width(), height: image.height() };
}

/** How the preview shows the photo: its box, zoom, pan, and turn. The crop frame is placed against a view. */
interface CropView {
  box: PreviewBox;
  scale: number;
  translate: { x: number; y: number };
  turn: number;
}

/** The view the preview is showing now, or `null` while there is no box. */
function liveView(box: PreviewBox | null = previewBox.value): CropView | null {
  if (!box) return null;
  return {
    box,
    scale: previewScaleValue.value,
    translate: previewTranslateValue.value,
    turn: previewRotationValue.value,
  };
}

/** A crop, as a part of the turned photo, placed on the screen in `view`. */
function placeIn(rect: CropRect, photo: Photo, view: CropView): ScreenRect {
  return cropToScreen(rect, photo, view.box, view.scale, view.translate, view.turn);
}

/** The part of the turned photo that a frame on the screen covers in `view`. */
function readFrom(frame: ScreenRect, photo: Photo, view: CropView): CropRect {
  return screenToCrop(frame, photo, view.box, view.scale, view.translate, view.turn);
}

/** The crop frame as a part of the photo, in the view it is showing, or `null` if there is no frame yet. */
function frameInPhoto(): CropRect | null {
  const frame = cropFrame.value;
  const photo = photoSize();
  const view = liveView();
  if (!frame || !photo || !view) return null;
  return readFrom(frame, photo, view);
}

/**
 * The area of the preview box the crop may use: inset by half a handle so every handle stays visible, and above the gestures row along
 * the bottom.
 */
function safeArea(box: PreviewBox): ScreenRect {
  const inset = HANDLE_HIT / 2;
  return { left: inset, top: inset, right: box.width - inset, bottom: box.height - previewControlsCover.value - inset };
}

/**
 * The part of the preview a release may leave the crop in: the whole box above the gestures row. A crop the drag left on the photo edge
 * stays put here, unlike the inset safe area, which would nudge it.
 */
function handleArea(box: PreviewBox): ScreenRect {
  return { left: 0, top: 0, right: box.width, bottom: box.height - previewControlsCover.value };
}

/**
 * The part of the photo the crop may cover in `view`. The crop is kept inside it, so it cannot be dragged past the top bar or under the
 * gestures row.
 */
function visibleCropBounds(photo: Photo, view: CropView): CropRect {
  return screenToCrop(safeArea(view.box), photo, view.box, view.scale, view.translate, view.turn);
}

/**
 * The part of the photo a corner drag may reach in `view`: the whole preview box, so the crop can grow past the gestures row while it is
 * dragged. Releasing settles it back inside the safe area, or zooms in on it when it has shrunk.
 */
function dragBounds(photo: Photo, view: CropView): CropRect {
  const { box } = view;
  const whole = { left: 0, top: 0, right: box.width, bottom: box.height };
  return screenToCrop(whole, photo, box, view.scale, view.translate, view.turn);
}

/** Sets the aspect the crop holds to, and refits the frame to it. */
export function chooseCropAspect(aspect: CropAspect) {
  cropAspect.next(aspect);
  const photo = photoSize();
  const view = liveView();
  const current = frameInPhoto();
  if (!photo || !view || !current) return;
  const bounds = visibleCropBounds(photo, view);
  const fitted = fitCropInTurn(
    fitCropToAspect(current, aspectRatio(aspect, photo), turnedPhoto(photo, view.turn), bounds),
    photo,
    view.turn,
    bounds,
  );
  cropFrame.next(placeIn(fitted, photo, view));
}

/** How much of the preview box a zoomed-to crop fills, as "Circle to zoom" does. */
const CROP_ZOOM_FILL = 0.9;
/** The crop, as a part of the photo, that a zoom to the crop is carrying the frame along for while the preview glides. */
let followingZoom: CropRect | null = null;

const cropArea = (rect: CropRect) => (rect.right - rect.left) * (rect.bottom - rect.top);

/**
 * Zooms the preview into a crop that a corner drag has made smaller than `start`, so the crop fills the view. Growing the crop does not
 * zoom. The frame rides along with the glide, so it stays on the same part of the photo. Returns whether it zoomed.
 */
function zoomToSmallerCrop(start: CropRect): boolean {
  const current = frameInPhoto();
  const frame = cropFrame.value;
  const view = liveView();
  if (!current || !frame || !view || cropArea(current) >= cropArea(start)) return false;
  const zoom = zoomToFrame(
    frame,
    view.box,
    view.scale,
    view.translate,
    view.turn,
    CROP_ZOOM_FILL,
    MAX_PREVIEW_ZOOM,
    safeArea(view.box),
  );
  if (!zoom) return false;
  followingZoom = current;
  springPreview(zoom);
  return true;
}

/**
 * Brings a crop a corner drag pushed past the edge of the preview back inside it. The photo slides under the frame, in whichever direction
 * it overflowed, so the crop keeps its size and the same part of the photo. Only a crop too big for the area zooms the preview out.
 * Returns whether it moved.
 */
function slideToSafeArea(): boolean {
  const current = frameInPhoto();
  const frame = cropFrame.value;
  const photo = photoSize();
  const view = liveView();
  if (!current || !frame || !photo || !view) return false;
  const area = handleArea(view.box);
  // The photo's own edges are the limit when they are inside the handle area, so the crop meets the photo's corner, not a gap short of it.
  const bounds = photoBoundsOnScreen(photo, view.box, view.scale, view.translate, view.turn);
  const onPhoto = {
    left: Math.max(area.left, bounds.left),
    top: Math.max(area.top, bounds.top),
    right: Math.min(area.right, bounds.right),
    bottom: Math.min(area.bottom, bounds.bottom),
  };
  const pan = slideFrameInto(frame, view.scale, view.translate, view.turn, onPhoto);
  if (!pan) return zoomOutToSafeArea();
  followingZoom = current;
  springPreview({ scale: view.scale, x: pan.x, y: pan.y });
  return true;
}

/**
 * Zooms the preview out so a crop a corner drag pushed past the gestures row fits the safe area at its own size. The frame rides along
 * with the glide, as it does for a zoom in. Returns whether it zoomed.
 */
function zoomOutToSafeArea(): boolean {
  const current = frameInPhoto();
  const frame = cropFrame.value;
  const view = liveView();
  if (!current || !frame || !view) return false;
  const zoom = zoomOutToFrame(
    frame,
    view.box,
    view.scale,
    view.translate,
    view.turn,
    previewZoomFloor.value,
    handleArea(view.box),
  );
  if (!zoom) return false;
  followingZoom = current;
  springPreview(zoom);
  return true;
}

/**
 * The crop frame and its four corner brackets, drawn over the preview. They sit where the frame is on screen: pinching moves the
 * photo under them, not them. Only the corner handles resize the frame; a drag inside it moves the photo (see the preview's gestures).
 * Dragging works in photo terms, so the frame follows the finger at the zoom it is at.
 */
export function CropGizmos() {
  const frame = useObservable(cropFrame$, cropFrame.value);
  const dragStart = useRef<CropRect | null>(null);
  // The gesture's own offset when the drag last re-anchored, so a slide can restart the drag from the crop it has reached.
  const dragOrigin = useRef({ dx: 0, dy: 0 });

  // Closing the tool mid-drag never reaches the handle's release, so the flag is cleared here.
  useEffect(() => () => cropHandleGrabbed.next(false), []);

  // Each drag reads the transform and the aspect when it moves, so the handlers never go stale.
  const handlers = useMemo(() => {
    const drag = (
      which: CropCorner,
      apply: (start: CropRect, dx: number, dy: number, photo: Photo, view: CropView) => CropRect,
    ): GestureResponderHandlers =>
      PanResponder.create({
        onStartShouldSetPanResponder: () => true,
        // A second finger is a pinch, which belongs to the preview, so let it take over.
        onPanResponderTerminationRequest: event => event.nativeEvent.touches.length > 1,
        onPanResponderGrant: () => {
          dragStart.current = frameInPhoto();
          dragOrigin.current = { dx: 0, dy: 0 };
          cropHandleGrabbed.next(true);
        },
        onPanResponderMove: (_event, gesture) => {
          const start = dragStart.current;
          const photo = photoSize();
          const view = liveView();
          const frame = cropFrame.value;
          if (!start || !photo || !view) return;
          const perPixel = dpPerPhotoPixel(photo, view.box, view.scale);
          const origin = dragOrigin.current;
          const next = apply(
            start,
            (gesture.dx - origin.dx) / perPixel,
            (gesture.dy - origin.dy) / perPixel,
            photo,
            view,
          );
          if (!frame) return;
          const placed = placeIn(next, photo, view);
          // The corner opposite the dragged one stays where it is. If the dragged edge runs past the handle area, the view zooms out
          // around that opposite corner until the crop fits, so the corner stays at the edge of the area.
          const fromLeft = which.endsWith('left');
          const fromTop = which.startsWith('top');
          const area = handleArea(view.box);
          const anchorX = fromLeft ? placed.right : placed.left;
          const anchorY = fromTop ? placed.bottom : placed.top;
          const roomX = fromLeft ? anchorX - area.left : area.right - anchorX;
          const roomY = fromTop ? anchorY - area.top : area.bottom - anchorY;
          const fit = Math.min(1, roomX / (placed.right - placed.left), roomY / (placed.bottom - placed.top));
          const nextScale = Math.max(previewZoomFloor.value, view.scale * fit);
          if (nextScale >= view.scale) {
            cropFrame.next(placed);
            return;
          }
          const pan = panToKeepPoint(
            { x: fromLeft ? start.right : start.left, y: fromTop ? start.bottom : start.top },
            photo,
            view.box,
            view.scale,
            view.translate,
            view.turn,
            nextScale,
          );
          previewScale.value.setValue(nextScale);
          previewTranslateX.value.setValue(pan.x);
          previewTranslateY.value.setValue(pan.y);
          cropFrame.next(placeIn(next, photo, { ...view, scale: nextScale, translate: pan }));
          dragStart.current = next;
          dragOrigin.current = { dx: gesture.dx, dy: gesture.dy };
        },
        onPanResponderRelease: () => {
          const start = dragStart.current;
          dragStart.current = null;
          cropHandleGrabbed.next(false);
          if (!start) return;
          if (zoomToSmallerCrop(start) || slideToSafeArea()) return;
        },
        onPanResponderTerminate: () => {
          dragStart.current = null;
          cropHandleGrabbed.next(false);
        },
      }).panHandlers;
    const corner = (which: CropCorner) =>
      drag(which, (start, dx, dy, photo, view) => {
        const bounds = dragBounds(photo, view);
        const moved = moveCropCorner(
          start,
          which,
          dx,
          dy,
          turnedPhoto(photo, view.turn),
          aspectRatio(cropAspect.value, photo),
          bounds,
        );
        return fitCropInTurn(moved, photo, view.turn, bounds);
      });
    return {
      'top-left': corner('top-left'),
      'top-right': corner('top-right'),
      'bottom-right': corner('bottom-right'),
      'bottom-left': corner('bottom-left'),
    };
  }, []);

  if (!frame) return null;

  const corners: Record<CropCorner, { x: number; y: number }> = {
    'top-left': { x: frame.left, y: frame.top },
    'top-right': { x: frame.right, y: frame.top },
    'bottom-right': { x: frame.right, y: frame.bottom },
    'bottom-left': { x: frame.left, y: frame.bottom },
  };

  // The layer is clipped to the preview, so a handle that reaches past the edge can neither show nor take touches under the top bar.
  // The four bands dim what the crop leaves out. They reach the preview's edges, and the frame sits inside the visible box.
  return (
    <View pointerEvents="box-none" style={[StyleSheet.absoluteFill, { overflow: 'hidden' }]}>
      <View pointerEvents="none" style={[gizmoStyles.shade, { left: 0, right: 0, top: 0, height: frame.top }]} />
      <View pointerEvents="none" style={[gizmoStyles.shade, { left: 0, right: 0, top: frame.bottom, bottom: 0 }]} />
      <View
        pointerEvents="none"
        style={[gizmoStyles.shade, { left: 0, width: frame.left, top: frame.top, height: frame.bottom - frame.top }]}
      />
      <View
        pointerEvents="none"
        style={[gizmoStyles.shade, { left: frame.right, right: 0, top: frame.top, height: frame.bottom - frame.top }]}
      />
      <View
        pointerEvents="none"
        style={[
          gizmoStyles.frame,
          { left: frame.left, top: frame.top, width: frame.right - frame.left, height: frame.bottom - frame.top },
        ]}
      />
      {[1, 2].map(third => {
        const x = frame.left + ((frame.right - frame.left) * third) / 3;
        const y = frame.top + ((frame.bottom - frame.top) * third) / 3;
        return (
          <Fragment key={third}>
            <View
              pointerEvents="none"
              style={[gizmoStyles.guide, { left: x, top: frame.top, width: 0, height: frame.bottom - frame.top }]}
            />
            <View
              pointerEvents="none"
              style={[gizmoStyles.guide, { top: y, left: frame.left, height: 0, width: frame.right - frame.left }]}
            />
          </Fragment>
        );
      })}
      {CORNERS.map(corner => (
        <View
          key={corner}
          {...handlers[corner]}
          style={[
            gizmoStyles.handle,
            { left: corners[corner].x - HANDLE_HIT / 2, top: corners[corner].y - HANDLE_HIT / 2 },
          ]}
        >
          <View pointerEvents="none" style={[gizmoStyles.bracket, BRACKET_SIDES[corner]]} />
        </View>
      ))}
    </View>
  );
}

/** Which two sides of the bracket box are drawn, and where the box sits in its handle, so its outer corner lands on the crop corner. */
const HALF = HANDLE_HIT / 2;
const BRACKET_SIDES: Record<CropCorner, ViewStyle> = {
  'top-left': { left: HALF, top: HALF, borderTopWidth: BRACKET_LINE, borderLeftWidth: BRACKET_LINE },
  'top-right': { left: HALF - BRACKET, top: HALF, borderTopWidth: BRACKET_LINE, borderRightWidth: BRACKET_LINE },
  'bottom-right': {
    left: HALF - BRACKET,
    top: HALF - BRACKET,
    borderBottomWidth: BRACKET_LINE,
    borderRightWidth: BRACKET_LINE,
  },
  'bottom-left': { left: HALF, top: HALF - BRACKET, borderBottomWidth: BRACKET_LINE, borderLeftWidth: BRACKET_LINE },
};

const SHAPE_BOX = 28;

/** The size of an aspect swatch's shape: it fits inside a square box, and freeform is a dashed square. */
function aspectShapeSize(ratio: number | null): { width: number; height: number } {
  if (ratio === null) return { width: SHAPE_BOX - 6, height: SHAPE_BOX - 6 };
  return ratio >= 1 ? { width: SHAPE_BOX, height: SHAPE_BOX / ratio } : { width: SHAPE_BOX * ratio, height: SHAPE_BOX };
}

export interface CropControlsProps {
  onBack: () => void;
  onApply: () => void;
}

/** The controls-row UI while the Crop tool is open: a back chip, the aspect swatches, and the button that cuts the photo. */
export function CropControls({ onBack, onApply }: CropControlsProps) {
  const { colors } = useTheme();
  const styles = useMemo(() => createStyles(colors), [colors]);
  const aspect = useObservable(cropAspect$, cropAspect.value);
  return (
    <View style={styles.controlRow}>
      <View style={styles.topRow}>
        <Pressable onPress={onBack} hitSlop={8} style={styles.backChip}>
          <ChevronLeft color={colors.textPrimary} size={18} />
        </Pressable>
        <Text numberOfLines={1} style={styles.hint}>
          Drag corners to resize, pinch to zoom
        </Text>
        <Pressable style={styles.applyButton} onPress={onApply}>
          <Text style={styles.applyButtonText}>Crop</Text>
        </Pressable>
      </View>
      <ScrollView
        horizontal
        showsHorizontalScrollIndicator={false}
        style={styles.aspectScroll}
        contentContainerStyle={styles.aspectRow}
      >
        {CROP_ASPECTS.map(option => {
          const selected = option.key === aspect;
          const photo = photoSize();
          const ratio = photo ? aspectRatio(option.key, photo) : 1;
          const shape = aspectShapeSize(ratio);
          return (
            <Pressable
              key={option.key}
              onPress={() => chooseCropAspect(option.key)}
              accessibilityRole="button"
              accessibilityLabel={option.label}
              accessibilityState={{ selected }}
              style={styles.aspectSwatch}
            >
              <View style={styles.aspectShapeBox}>
                <View
                  style={[
                    styles.aspectShape,
                    ratio === null && styles.aspectShapeFreeform,
                    selected && styles.aspectShapeChosen,
                    shape,
                  ]}
                />
              </View>
              <Text style={[styles.aspectLabel, selected && styles.aspectLabelChosen]}>{option.label}</Text>
            </Pressable>
          );
        })}
      </ScrollView>
    </View>
  );
}

/**
 * Puts the crop frame around the whole photo as it sits at `scale` and `translate` in the current turn, shrunk if needed to stay in the
 * safe area. The default is the photo at 1x, centred.
 */
function placeFullFrame(scale: number, translate: { x: number; y: number }) {
  const photo = photoSize();
  const view = liveView();
  if (!photo || !view) {
    cropFrame.next(null);
    return;
  }
  const target: CropView = { ...view, scale, translate };
  const bounds = visibleCropBounds(photo, target);
  cropFrame.next(placeIn(fitCropInTurn(FULL_CROP, photo, target.turn, bounds), photo, target));
}

/** The preview's zoom and pan limit while the crop is open: the photo must keep covering the frame, so no edge shows past it. */
function keepFrameCovered(scale: number, translate: { x: number; y: number }) {
  const frame = cropFrame.value;
  const photo = photoSize();
  const box = previewBox.value;
  if (!frame || !photo || !box) return { scale, x: translate.x, y: translate.y };
  return coverCropFrame(frame, photo, box, scale, translate);
}

/**
 * Keeps the crop frame where it is on the screen while the photo moves, zooms or turns under it, and shrinks it only if the photo no
 * longer covers it. The frame is read in the photo as it was placed (`from`), then placed again in `view`.
 */
function refitFrame(view: CropView, from: PreviewBox = view.box, clamp = true) {
  const frame = cropFrame.value;
  const photo = photoSize();
  if (!frame || !photo) return;
  const inPhoto = readFrom(frame, photo, { ...view, box: from });
  // A held handle may grow the crop past the gestures row; release settles it back.
  if (!clamp || cropHandleGrabbed.value) {
    cropFrame.next(placeIn(inPhoto, photo, view));
    return;
  }
  const bounds = visibleCropBounds(photo, view);
  cropFrame.next(placeIn(fitCropInTurn(inPhoto, photo, view.turn, bounds), photo, view));
}

/**
 * Wires up the Crop tool on the editor controller: starting from the full photo, upright, at 1x each time it opens; cutting the
 * photo on apply; and restoring rotation and zoom when it closes.
 */
export function CropToolController() {
  useEffect(() => {
    let open = false;
    let lastBox: PreviewBox | null = null;
    // The turn the photo had when the crop opened, so closing without applying puts it back.
    let turnAtOpen: number | null = null;
    const subscriptions = [
      // The frame is kept in screen space, so when the box changes size it is re-measured in photo terms and placed again.
      previewBox$.subscribe(box => {
        const previous = lastBox;
        lastBox = box;
        const view = liveView(box);
        if (!open || !view || !photoSize()) return;
        if (!cropFrame.value || !previous) {
          placeFullFrame(view.scale, view.translate);
          return;
        }
        refitFrame(view, previous);
      }),

      // The gestures row grows and shrinks (its hint text comes and goes), which moves the bottom of the safe area.
      previewControlsCover$.subscribe(() => {
        const view = liveView();
        if (open && view) refitFrame(view);
      }),

      isCropToolFocused$.subscribe(focused => {
        if (focused === open) return;
        open = focused;
        // The preview rests at 1x unless the crop is open. Opening shrinks the photo just enough to fit above the gestures row, centred
        // there, so every edge of the photo can be reached by a handle.
        let rest = { x: 0, y: 0, scale: 1 };
        if (focused) {
          cropAspect.next('freeform');
          turnAtOpen = previewRotationValue.value;
          const box = previewBox.value;
          const photo = photoSize();
          const cover = previewControlsCover.value;
          if (box && photo) {
            const floor = cropZoomFloor(photo, box, cover, turnAtOpen);
            const centre = panToCentreAboveBar(cover, turnAtOpen);
            previewZoomFloor.next(floor);
            rest = { ...centre, scale: floor };
            placeFullFrame(floor, centre);
          } else {
            placeFullFrame(1, { x: 0, y: 0 });
          }
          previewLimit.next(keepFrameCovered);
        } else {
          if (turnAtOpen !== null && Math.abs(previewRotationValue.value - turnAtOpen) > TURN_EPSILON) {
            previewRotationGlide.next(turnAtOpen);
          }
          turnAtOpen = null;
          followingZoom = null;
          previewLimit.next(null);
          previewZoomFloor.next(MIN_PREVIEW_ZOOM);
          cropFrame.next(null);
        }
        springPreview(rest);
      }),

      // The frame stays on the same part of the screen while the photo zooms, pans or turns under it. A zoom to a smaller crop is the
      // exception: the frame rides along with the glide so it stays on the same part of the photo until the glide ends.
      combineLatest([previewScaleValue$, previewTranslateValue$, previewRotationValue$, previewMoving$]).subscribe(
        ([scale, translate, turn, moving]) => {
          const box = previewBox.value;
          const photo = photoSize();
          if (!open || !box || !photo) return;
          const view: CropView = { box, scale, translate, turn };
          const rect = followingZoom;
          if (rect) {
            cropFrame.next(placeIn(rect, photo, view));
            if (!moving) followingZoom = null;
            return;
          }
          // A pan or zoom moves the photo under the frame and leaves the frame's part of the photo alone. Settling into the safe area
          // waits for release, so the photo does not shift while the finger is down.
          refitFrame(view, view.box, false);
        },
      ),

      cropApplyRequested$.subscribe(() => {
        // The crop is cut from the full-size photo, so it must have this edit's full-size version first.
        flushFullReplay();
        // Until the history is built the image is a project's saved picture, and cutting it would be lost.
        if (!currentImage.value || !historyReady.value) return;
        const rect = frameInPhoto();
        const turn = previewRotationValue.value;
        const turned = Math.abs(turn) > TURN_EPSILON;
        if (rect && (turned || !isFullCrop(rect))) {
          commitCrop(rect, turned ? turn : 0);
          // The turn is now part of the picture, so the preview must not turn it again.
          previewRotation.value.setValue(0);
        }
        turnAtOpen = null;
        focusedControlKey.next(null);
      }),
    ];
    return () => {
      subscriptions.forEach(subscription => subscription.unsubscribe());
      previewLimit.next(null);
      previewZoomFloor.next(MIN_PREVIEW_ZOOM);
    };
  }, []);

  return null;
}

const gizmoStyles = StyleSheet.create({
  frame: { position: 'absolute', borderWidth: 1.5, borderColor: 'rgba(255,255,255,0.85)' },
  guide: {
    position: 'absolute',
    borderColor: 'rgba(255,255,255,0.65)',
    borderLeftWidth: 1.5,
    borderTopWidth: 1.5,
  },
  shade: { position: 'absolute', backgroundColor: 'rgba(0,0,0,0.5)' },
  handle: { position: 'absolute', width: HANDLE_HIT, height: HANDLE_HIT },
  bracket: {
    position: 'absolute',
    width: BRACKET,
    height: BRACKET,
    borderColor: '#fff',
  },
});

function createStyles(colors: ThemeColors) {
  return StyleSheet.create({
    controlRow: { alignItems: 'center', gap: 8, paddingHorizontal: 16 },
    topRow: { flexDirection: 'row', alignItems: 'center', gap: 10, alignSelf: 'stretch' },
    backChip: {
      width: 28,
      height: 28,
      borderRadius: 14,
      backgroundColor: colors.chip,
      alignItems: 'center',
      justifyContent: 'center',
    },
    aspectScroll: { alignSelf: 'stretch', flexGrow: 0 },
    aspectRow: { gap: 8, paddingVertical: 4, paddingHorizontal: 4 },
    aspectSwatch: { alignItems: 'center', gap: 3, width: 64 },
    aspectShapeBox: { width: SHAPE_BOX, height: SHAPE_BOX, alignItems: 'center', justifyContent: 'center' },
    aspectShape: { borderWidth: 1.5, borderColor: colors.textTertiary, borderRadius: 3 },
    aspectShapeFreeform: { borderStyle: 'dashed' },
    aspectShapeChosen: { borderWidth: 2.5, borderColor: colors.accent },
    aspectLabel: { color: colors.textSecondary, fontSize: 11, fontWeight: '500', textAlign: 'center' },
    aspectLabelChosen: { color: colors.textPrimary, fontWeight: '700' },
    hint: { flex: 1, color: colors.textSecondary, fontSize: 12 },
    applyButton: {
      height: 28,
      justifyContent: 'center',
      paddingHorizontal: 14,
      borderRadius: 14,
      backgroundColor: colors.accent,
    },
    applyButtonText: { color: '#fff', fontSize: 14, fontWeight: '700' },
  });
}
