import { useEffect, useMemo, useRef, useState } from 'react';
import { PanResponder, StyleSheet, View } from 'react-native';
import Svg, { Line } from 'react-native-svg';
import { useObservable } from 'react-rx';
import { GRADIENT_AREA, GRADIENT_AT, gradientFromPhoto, gradientShown, gradientValues, withCustomGradient } from '@/src/effects/gradient-overlay';
import { useSliderPreview } from '@/src/hooks/useSliderPreview';
import { computePreviewImagePoint, computePreviewScreenPoint } from '@/src/lib/editor/preview-coordinates';
import { leafControls, sliderValues, type SliderControl } from '@/src/lib/project/edit-sections';
import { GRADIENT_SETTINGS_KEY } from '@/src/lib/project/edit-sections/keys';
import { gradientControl } from '@/src/lib/project/edit-sections/overlays';
import { missingModels } from '@/src/lib/ai/ai-model-availability';
import { PERSON_DETECTION } from '@/src/lib/ai/faces';
import { devDebug } from '@/src/lib/log';
import { aiModelsRequested } from '@/src/state/ai-models';
import { gradientChoiceRequested$, sliderCommitRequested } from '@/src/state/commands';
import { pickGradient } from '@/src/state/gradient-picker';
import { adjustments, adjustments$ } from '@/src/state/edits';
import {
  previewBox,
  previewBox$,
  previewRotationValue,
  previewRotationValue$,
  previewScaleValue,
  previewScaleValue$,
  previewTranslateValue,
  previewTranslateValue$,
} from '@/src/state/gestures';
import { currentImage, currentImage$, editBaseImage } from '@/src/state/session';

/** Size (dp) of the square a handle can be grabbed in, and of the dot drawn in it. */
const HANDLE_HIT = 52;
const DOT = 24;

/** The slider that holds the gradient's settings: what is open while the gradient is, and what the handles save into. */
const settings = leafControls([gradientControl]).find(
  (control): control is SliderControl => control.kind === 'slider' && control.key === GRADIENT_SETTINGS_KEY,
)!;

/** Where the handles are among the saved values: the start's x and y, then the end's. */
const HANDLES_AT = GRADIENT_AT.startX;

/**
 * The two handles of the open gradient, drawn over the photo: the dot where the gradient starts and the dot where it ends, joined by a
 * line. Dragging a dot moves that end of the gradient, which the photo follows as the finger moves (the same live preview a slider
 * uses) and which is saved when the finger lifts. A touch anywhere else goes to the preview, so the photo can still be pinched and
 * panned. This is the only way the gradient's direction and length are set.
 */
export function GradientHandles() {
  const edits = useObservable(adjustments$, adjustments.value);
  const box = useObservable(previewBox$, previewBox.value);
  const scale = useObservable(previewScaleValue$, previewScaleValue.value);
  const translate = useObservable(previewTranslateValue$, previewTranslateValue.value);
  const turn = useObservable(previewRotationValue$, previewRotationValue.value);
  const image = useObservable(currentImage$, currentImage.value);
  const onLiveChange = useSliderPreview(settings);

  // What is saved, with the handles at their start places until they are moved.
  const saved = gradientValues(sliderValues(settings, edits[GRADIENT_SETTINGS_KEY]));
  // While a finger is down the dots follow it, and these are all the values with the moved handle, ready to save. They stay until the
  // save reaches the edits (below), so a dot does not jump back to its old place for a moment when the finger lifts.
  const [dragged, setDragged] = useState<number[] | null>(null);
  useEffect(() => setDragged(null), [edits[GRADIENT_SETTINGS_KEY]]);
  const shown = dragged ?? saved;

  // The responders live as long as the component does, so they read the latest values through refs.
  const latest = useRef({ saved, onLiveChange });
  latest.current = { saved, onLiveChange };
  // The cards of the colors that are not ready-made looks. From photo works the photo's colors out and saves them as the person's own gradient.
  // Custom opens the gradient editor: it starts from the gradient on the photo now (a ready-made look, or the one made before), shows each
  // change on the photo as it is made, and saves the gradient on OK. Cancel puts the photo back as it was, and nothing is saved, so it adds
  // no step to undo.
  useEffect(() => {
    const subscription = gradientChoiceRequested$.subscribe(async request => {
      const before = latest.current.saved;
      if (request.kind === 'area') {
        const save = () =>
          sliderCommitRequested.next({ control: settings, value: before.map((value, index) => (index === GRADIENT_AT.area ? request.area : value)) });
        // The people are found by models that are not on every device, so they are offered when this needs them. Until the cutout is
        // found the gradient is not laid (see `gradientOverlay`), and the editor draws it when it is.
        if (request.area !== GRADIENT_AREA.whole && missingModels([PERSON_DETECTION]).length > 0) {
          aiModelsRequested.next({ title: 'Find the people', roles: [PERSON_DETECTION], onReady: save });
        } else save();
        return;
      }
      if (request.kind === 'photo') {
        // The photo the edits start from, so the colors are the photo's and not those of an edit made on it.
        const photo = editBaseImage.value ?? currentImage.value;
        if (!photo) return;
        const started = Date.now();
        const found = gradientFromPhoto(photo, request.scheme);
        devDebug('gradient', 'colors from the photo,', request.scheme, ':', found?.colors.length ?? 0, 'in', Date.now() - started, 'ms');
        if (found) sliderCommitRequested.next({ control: settings, value: withCustomGradient(before, found) });
        return;
      }
      const picked = await pickGradient({
        title: 'Make your own gradient',
        value: gradientShown(before),
        onChange: gradient => latest.current.onLiveChange?.(withCustomGradient(before, gradient)),
      });
      if (picked) sliderCommitRequested.next({ control: settings, value: withCustomGradient(before, picked) });
      else latest.current.onLiveChange?.(before);
    });
    return () => subscription.unsubscribe();
  }, []);

  const responders = useMemo(() => {
    const handle = (end: 0 | 1) => {
      let values: number[] = [];
      let from = { x: 0, y: 0 };
      // A tap that never moves the dot saves nothing, so it does not add a step to undo.
      let moved = false;
      const save = () => {
        if (moved) sliderCommitRequested.next({ control: settings, value: values });
        moved = false;
      };
      return PanResponder.create({
        onStartShouldSetPanResponder: () => true,
        // A second finger is a pinch, which belongs to the preview.
        onPanResponderTerminationRequest: event => event.nativeEvent.touches.length > 1,
        onPanResponderGrant: () => {
          moved = false;
          values = [...latest.current.saved];
          const photo = currentImage.value;
          const view = previewBox.value;
          if (!photo || !view) return;
          from = computePreviewScreenPoint(
            photo.width(),
            photo.height(),
            view,
            previewScaleValue.value,
            previewTranslateValue.value,
            { x: values[HANDLES_AT + end * 2], y: values[HANDLES_AT + end * 2 + 1] },
            previewRotationValue.value,
          );
        },
        onPanResponderMove: (_event, gesture) => {
          const photo = currentImage.value;
          const view = previewBox.value;
          if (!photo || !view) return;
          // Held at the photo's edge when the finger leaves it, so the end of the gradient is always on the photo.
          const point = computePreviewImagePoint(
            photo.width(),
            photo.height(),
            view,
            previewScaleValue.value,
            previewTranslateValue.value,
            { x: from.x + gesture.dx, y: from.y + gesture.dy },
            previewRotationValue.value,
            true,
          );
          if (!point) return;
          moved = true;
          values = [...values];
          values[HANDLES_AT + end * 2] = point.x / photo.width();
          values[HANDLES_AT + end * 2 + 1] = point.y / photo.height();
          setDragged(values);
          latest.current.onLiveChange?.(values);
        },
        onPanResponderRelease: save,
        // Another responder took the touch (a pinch): what was moved so far is kept.
        onPanResponderTerminate: save,
      }).panHandlers;
    };
    return [handle(0), handle(1)];
  }, []);

  if (!box || !image || image.width() === 0 || image.height() === 0) return null;
  const places = [0, 1].map(end =>
    computePreviewScreenPoint(
      image.width(),
      image.height(),
      box,
      scale,
      translate,
      { x: shown[HANDLES_AT + end * 2], y: shown[HANDLES_AT + end * 2 + 1] },
      turn,
    ),
  );

  // The layer is clipped to the preview, as the crop's is, so a dot cannot be under the top bar.
  return (
    <View pointerEvents="box-none" style={[StyleSheet.absoluteFill, styles.layer]}>
      <Svg pointerEvents="none" style={StyleSheet.absoluteFill}>
        {/* A dark line under the white one, so the line shows on light and dark parts of the photo alike. */}
        <Line x1={places[0].x} y1={places[0].y} x2={places[1].x} y2={places[1].y} stroke="rgba(0,0,0,0.45)" strokeWidth={4} strokeLinecap="round" />
        <Line x1={places[0].x} y1={places[0].y} x2={places[1].x} y2={places[1].y} stroke="#ffffff" strokeWidth={2} strokeLinecap="round" />
      </Svg>
      {places.map((place, end) => (
        <View
          key={end}
          {...responders[end]}
          accessibilityRole="adjustable"
          accessibilityLabel={end === 0 ? 'Gradient start' : 'Gradient end'}
          style={[styles.hit, { left: place.x - HANDLE_HIT / 2, top: place.y - HANDLE_HIT / 2 }]}
        >
          <View pointerEvents="none" style={[styles.dot, end === 0 ? styles.start : styles.end]} />
        </View>
      ))}
    </View>
  );
}

const styles = StyleSheet.create({
  layer: { overflow: 'hidden' },
  hit: { position: 'absolute', width: HANDLE_HIT, height: HANDLE_HIT, alignItems: 'center', justifyContent: 'center' },
  dot: {
    width: DOT,
    height: DOT,
    borderRadius: DOT / 2,
    borderWidth: 3,
    borderColor: '#ffffff',
    shadowColor: '#000000',
    shadowOpacity: 0.5,
    shadowRadius: 3,
    elevation: 4,
  },
  // The start is a solid dot and the end a ring, so which is which can be told at a glance.
  start: { backgroundColor: '#ffffff' },
  end: { backgroundColor: 'rgba(255,255,255,0.15)' },
});
