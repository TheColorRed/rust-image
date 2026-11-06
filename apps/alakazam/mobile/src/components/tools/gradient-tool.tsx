import { HandlesOverlay } from '@/src/components/editor/handles-overlay';
import { Handles, HandleShape, Joined, type ImagePreview } from '@alakazam/mobile';
import { useEffect, useRef } from 'react';
import { PixelRatio } from 'react-native';
import { useObservable } from 'react-rx';
import { GRADIENT_AREA, GRADIENT_AT, gradientFromPhoto, gradientShown, gradientValues, withCustomGradient } from '@/src/effects/gradient-overlay';
import { useSliderPreview } from '@/src/hooks/useSliderPreview';
import { useOwning } from '@/src/lib/native/native-handles';
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
import { previewBox, previewBox$ } from '@/src/state/gestures';
import { currentImage, currentImage$, editBaseImage } from '@/src/state/session';

/** The slider that holds the gradient's settings: what is open while the gradient is, and what the handles save into. */
const settings = leafControls([gradientControl]).find(
  (control): control is SliderControl => control.kind === 'slider' && control.key === GRADIENT_SETTINGS_KEY,
)!;

/** Where the handles are among the saved values: the start's x and y, then the end's. */
const HANDLES_AT = GRADIENT_AT.startX;

/**
 * The two handles of the open gradient, drawn over the photo: the dot where the gradient starts and the ring where it ends, joined by a line. Dragging
 * one moves that end of the gradient, which the photo follows as the finger moves (the same live preview a slider uses) and which is saved when the
 * finger lifts. The handles and everything about them (grabbing, moving, staying on the photo as it is zoomed, panned and turned) are the native
 * `Handles`, the same component a perspective or straighten tool is made of; this says where the gradient's handles are and what moving them does.
 * `view` is the preview they follow. This is the only way the gradient's direction and length are set.
 */
export function GradientHandles({ view }: { view: ImagePreview | null }) {
  const edits = useObservable(adjustments$, adjustments.value);
  const box = useObservable(previewBox$, previewBox.value);
  const onLiveChange = useSliderPreview(settings);

  // What is saved, with the handles at their start places until they are moved.
  const saved = gradientValues(sliderValues(settings, edits[GRADIENT_SETTINGS_KEY]));
  // The handles' places among the saved values: the start's x and y, then the end's.
  const placesOf = (values: readonly number[]) => values.slice(HANDLES_AT, HANDLES_AT + 4);
  const handles = useOwning(keep => keep(new Handles(placesOf(saved), [HandleShape.Dot, HandleShape.Ring], Joined.Line, true, PixelRatio.get())), []);

  // The callbacks are read through a ref, so the subscription and the touches below live as long as the component does.
  const latest = useRef({ saved, onLiveChange });
  latest.current = { saved, onLiveChange };
  // The saved places can come from somewhere else (undo, a reset), and then the handles show them. A handle under a finger is left where it is.
  const savedPlaces = placesOf(saved).join(',');
  useEffect(() => {
    handles?.setPoints(placesOf(latest.current.saved));
  }, [handles, savedPlaces]);
  /** All the saved values, with the handles' places from the handles. */
  const withPlaces = (points: number[]) => {
    const values = [...latest.current.saved];
    points.forEach((point, index) => (values[HANDLES_AT + index] = point));
    return values;
  };

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

  if (!box || !handles) return null;
  return (
    <HandlesOverlay
      handles={handles}
      view={view}
      box={box}
      onMove={points => latest.current.onLiveChange?.(withPlaces(points))}
      // A tap that never moves a handle saves nothing, so it does not add a step to undo.
      onEnd={(points, moved) => {
        if (moved) sliderCommitRequested.next({ control: settings, value: withPlaces(points) });
      }}
    />
  );
}
