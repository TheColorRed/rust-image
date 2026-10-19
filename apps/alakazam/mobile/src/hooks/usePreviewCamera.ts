import { editorPreview } from '@/src/lib/editor/editor-preview';
import {
  PREVIEW_GLIDE_MS,
  previewGlide,
  previewMoving,
  previewRotation,
  previewRotationGlide,
  previewRotationValue,
  previewScale,
  previewScaleValue,
  previewTouch,
  previewTranslateValue,
  previewTranslateX,
  previewTranslateY,
  type PreviewBox,
} from '@/src/state/gestures';
import type { ImagePreview } from '@alakazam/mobile';
import { useEffect, useRef, type RefObject } from 'react';
import { animationFrameScheduler, combineLatest } from 'rxjs';
import { auditTime } from 'rxjs/operators';

type Viewport = [number, number, number, number];
// The camera: the middle of the view as a point on the photo (0 to 1) and the zoom. It is kept in these terms, not as the part
// of the photo that shows, because the box changes shape when a control opens: the same camera still points at the same place
// (and zoom 1 is still the whole photo), where a stored viewport would show something else.
type Camera = { x: number; y: number; zoom: number };

type Glide = { from: Camera; target: Camera; started: number; done: boolean };

// The same path as Vessel's tween (`Camera`): the size eases in log space and the center moves in a line, so the outlines stay
// on the photo.
const blend = (from: Camera, to: Camera, share: number): Camera => ({
  x: from.x + (to.x - from.x) * share,
  y: from.y + (to.y - from.y) * share,
  zoom: from.zoom * Math.pow(to.zoom / from.zoom, share),
});

const easeInOutCubic = (t: number) => (t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2);

const setViewports = (viewport: Viewport, baseView: ImagePreview | null) => {
  baseView?.setViewport(...viewport);
  // The live preview that is open now: another effect in the same commit may have replaced it (`ensure`) and let go of the old one.
  editorPreview.view.value?.setViewport(...viewport);
};

const setRotations = (angle: number, baseView: ImagePreview | null) => {
  baseView?.setRotation(angle);
  editorPreview.view.value?.setRotation(angle);
};

/**
 * Drives the GPU views' camera from the preview's zoom, pan and turn values, and runs the eased moves
 * (`previewGlide`, `previewRotationGlide`). The native views always fill the preview box: zoom and pan only move the camera, so the
 * photo is complete at every frame of a gesture, and the part past its edges is black.
 */
export function usePreviewCamera({
  photo,
  liveShown,
  box,
  baseView,
  liveView,
  baseViewRef,
}: {
  photo: { width: number; height: number } | null;
  liveShown: { width: number; height: number } | null;
  box: PreviewBox | null;
  baseView: ImagePreview | null;
  liveView: unknown;
  baseViewRef: RefObject<ImagePreview | null>;
}) {
  // The move of the preview that was asked for last. A finger on the photo clears it. It lives outside the effect, as parts of the
  // photo and not as pixels, so it outlives it: the effect runs again when the box changes size (a control opening) or a view is
  // replaced, and the move carries on, or stays where it landed, in the new box.
  const glideRef = useRef<Glide | null>(null);

  useEffect(() => {
    const shown = liveShown ?? photo;
    if (!shown || !box) return;
    const fit = Math.min(box.width / shown.width, box.height / shown.height);
    const size = { width: Math.max(1, shown.width * fit), height: Math.max(1, shown.height * fit) };
    const cameraOf = (zoom: number, move: { x: number; y: number }): Camera => ({
      x: 0.5 - move.x / (zoom * size.width),
      y: 0.5 - move.y / (zoom * size.height),
      zoom,
    });
    // The part of the photo (0 to 1) the box shows.
    const viewportOf = ({ x, y, zoom }: Camera): Viewport => {
      const width = box.width / (zoom * size.width);
      const height = box.height / (zoom * size.height);
      return [x - width / 2, y - height / 2, width, height];
    };
    // The zoom and pan that put the camera there.
    const poseOf = ({ x, y, zoom }: Camera) => ({ zoom, x: -(x - 0.5) * zoom * size.width, y: -(y - 0.5) * zoom * size.height });
    const base = () => baseViewRef.current;

    // While a move is under way Vessel makes the photo's frames itself, so the values below must not push the camera around.
    let gliding = false;
    let glideFrame: number | null = null;
    const stopGlide = () => {
      if (glideFrame !== null) cancelAnimationFrame(glideFrame);
      glideFrame = null;
      gliding = false;
    };
    const endInteraction = () => previewMoving.next(false);
    const subscription = combineLatest([previewScaleValue, previewTranslateValue])
      .pipe(auditTime(0, animationFrameScheduler)) // scale, x and y change one after another: send the camera once per frame
      .subscribe(([zoom, move]) => {
        if (gliding) return;
        setViewports(viewportOf(cameraOf(zoom, move)), base());
      });

    // While a turn glides, Vessel's tween turns the views and the outlines follow the same curve from here.
    let turning = false;
    let turnFrame: number | null = null;
    const stopTurn = () => {
      if (turnFrame !== null) cancelAnimationFrame(turnFrame);
      turnFrame = null;
      turning = false;
    };
    // Two fingers turn the photo on the GPU: the views take the angle (a frame at a time) and draw the photo turned.
    subscription.add(
      previewRotationValue.pipe(auditTime(0, animationFrameScheduler)).subscribe(angle => {
        if (turning) return;
        setRotations(angle, base());
      }),
    );
    subscription.add(
      previewRotationGlide.subscribe(to => {
        stopTurn();
        const from = previewRotationValue.value;
        turning = true;
        base()?.tweenRotation(to, PREVIEW_GLIDE_MS);
        editorPreview.view.value?.tweenRotation(to, PREVIEW_GLIDE_MS);
        const started = Date.now();
        const step = () => {
          const t = Math.min(1, (Date.now() - started) / PREVIEW_GLIDE_MS);
          previewRotation.value.setValue(from + (to - from) * easeInOutCubic(t));
          if (t < 1) turnFrame = requestAnimationFrame(step);
          else {
            // A whole number of turns is the same picture as upright, so it is set to that.
            const wrapped = to % (2 * Math.PI);
            stopTurn();
            previewRotation.value.setValue(wrapped);
            setRotations(wrapped, base());
          }
        };
        turnFrame = requestAnimationFrame(step);
      }),
    );

    const runGlide = () => {
      const glide = glideRef.current;
      if (!glide) return;
      stopGlide();
      const moved = (share: number) => {
        const pose = poseOf(blend(glide.from, glide.target, share));
        previewScale.value.setValue(pose.zoom);
        previewTranslateX.value.setValue(pose.x);
        previewTranslateY.value.setValue(pose.y);
      };
      const remaining = PREVIEW_GLIDE_MS - (Date.now() - glide.started);
      if (glide.done || remaining <= 0) {
        glide.done = true;
        moved(1);
        endInteraction();
        return;
      }
      gliding = true;
      // The views ease in log space along the same path, over the time that is left.
      const target = viewportOf(glide.target);
      base()?.tweenViewport(...target, remaining);
      editorPreview.view.value?.tweenViewport(...target, remaining);
      const step = () => {
        const t = Math.min(1, (Date.now() - glide.started) / PREVIEW_GLIDE_MS);
        moved(easeInOutCubic(t));
        if (t < 1) glideFrame = requestAnimationFrame(step);
        else {
          glide.done = true;
          stopGlide();
          endInteraction();
        }
      };
      glideFrame = requestAnimationFrame(step);
    };
    subscription.add(
      previewGlide.subscribe(to => {
        if (!to) {
          stopGlide();
          endInteraction();
          glideRef.current = null;
          return;
        }
        previewMoving.next(true);
        previewScale.value.stopAnimation();
        previewTranslateX.value.stopAnimation();
        previewTranslateY.value.stopAnimation();
        glideRef.current = {
          from: cameraOf(previewScaleValue.value, previewTranslateValue.value),
          target: cameraOf(to.scale ?? previewScaleValue.value, to),
          started: Date.now(),
          done: false,
        };
        runGlide();
      }),
    );

    // A finger on the photo takes a turn or a move over.
    subscription.add(
      previewTouch.subscribe(event => {
        if (event.type !== 'grant') return;
        if (turning) {
          stopTurn();
          setRotations(previewRotationValue.value, base());
        }
        const wasGliding = gliding;
        stopGlide();
        endInteraction();
        glideRef.current = null;
        if (wasGliding) setViewports(viewportOf(cameraOf(previewScaleValue.value, previewTranslateValue.value)), base());
      }),
    );

    // Carries on a move that was under way, or keeps the place a finished one chose when the box is a different size now.
    runGlide();
    return () => {
      stopGlide();
      stopTurn();
      subscription.unsubscribe();
    };
    // The views are dependencies so the camera is pushed again when one is replaced.
  }, [photo, liveShown, box, baseView, liveView, baseViewRef]);
}
