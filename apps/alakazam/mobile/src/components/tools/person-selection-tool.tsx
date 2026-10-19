import { type Image as AbraPixels } from '@abra/core/image';
import { findPeopleAsync, personDetectionDownloaded } from '@alakazam/mobile';
import { aiProgress } from '@/src/state/ai-progress';
import { useEffect, useMemo } from 'react';
import { Pressable, StyleSheet, Text } from 'react-native';
import { useObservable } from 'react-rx';
import { combineLatest } from 'rxjs';
import { distinctUntilChanged, map } from 'rxjs/operators';
import { detectSkin, peopleNotHealed, personAt } from '@/src/lib/ai/analysis';
import { timeline } from '@/src/state/history';
import { replacedSpots } from '@/src/lib/project/timeline';
import { findFaceBoxes } from '@/src/lib/ai/faces';
import { computePreviewImagePoint } from '@/src/lib/editor/preview-coordinates';
import { editorPreview } from '@/src/lib/editor/editor-preview';
import { errorMessage } from '@/src/lib/utils/error-message';
import { useTheme } from '@/src/lib/ui/theme';
import { aiModelsVersion$ } from '@/src/state/ai-models';
import { faceBoxes$, people } from '@/src/state/analysis';
import { isFaceControlFocused$, isPersonControlFocused$ } from '@/src/state/edits';
import {
  personOutlinesVisible,
  personSelection,
  personSelection$,
  selectionNeeded,
} from '@/src/state/person-selection';
import { openProject } from '@/src/state/project-session';
import { previewBox, previewPressed$, previewRotationValue, previewScaleValue, previewTranslateValue, springPreview } from '@/src/state/gestures';
import { currentImage, currentImage$, editBaseImage, editBaseImage$, originalImage } from '@/src/state/session';

/** Zoomed to a face, the shorter side of the preview is this many times the face, and the zoom never goes past `MAX_FACE_ZOOM`. */
const FACE_FILL = 1.6;
const MAX_FACE_ZOOM = 8;

/** Detects people while the selector is open and maps image taps to the Rust-owned instance masks. */
export function PersonSelectionToolController() {
  useEffect(() => {
    let requestId = 0;
    const subscriptions = [
      combineLatest([isPersonControlFocused$, currentImage$, editBaseImage$, aiModelsVersion$])
        .pipe(
          map(([isOpen, image, editBase, version]) => [isOpen, editBase ?? image, version] as const),
          distinctUntilChanged(
            ([previousOpen, previousImage, previousVersion], [isOpen, image, version]) =>
              previousOpen === isOpen && previousImage === image && previousVersion === version,
          ),
        )
        .subscribe(([isOpen, image]) => {
          const currentRequest = ++requestId;
          personOutlinesVisible.next(isOpen);
          if (!isOpen) {
            if (personSelection.value.focused) {
              personSelection.next({ ...personSelection.value, focused: false, loading: false });
            }
            return;
          }

          if (!image) {
            personSelection.next({
              ...personSelection.value,
              focused: true,
              loading: false,
              error: 'No image is open.',
            });
            return;
          }
          // The skin model runs now that a control that uses the skin is open, and not when the photo opens. Without it the
          // controls find the skin by its color.
          detectSkin(originalImage.value ?? image).catch(error => {
            if (__DEV__) console.debug('[image] skin detection unavailable:', errorMessage(error));
          });
          // Without the body model there is nothing to detect; skin effects apply to the whole photo. Detection starts once
          // the model is downloaded.
          if (!personDetectionDownloaded() && !openProject.value?.people) {
            personSelection.next({ ...personSelection.value, focused: true, loading: false, error: null });
            return;
          }
          personSelection.next({
            ...personSelection.value,
            focused: true,
            loading: true,
            error: null,
          });
          // The people were found already when the photo is a saved project, or when the selector was opened before.
          const finding = people.value ? Promise.resolve(people.value) : findPeopleAsync(image);
          if (!people.value) {
            aiProgress.next({ ...aiProgress.value, people: null });
            finding.then(
              () => aiProgress.next({ ...aiProgress.value, people: 1 }),
              () => {
                // A run that failed did no work, so it leaves nothing on the bar.
                const { people: _failed, ...others } = aiProgress.value;
                aiProgress.next(others);
              },
            );
          }
          finding.then(
            found => {
              if (requestId !== currentRequest || !personSelection.value.focused) {
                if (found !== people.value) found.forEach(person => (person.mask as unknown as AbraPixels).uniffiDestroy());
                return;
              }
              people.next(found);
              // Someone a tool has removed from the photo is not there to pick any more.
              const { entries, cursor } = timeline.value;
              const present = peopleNotHealed(found, replacedSpots(entries, cursor), (originalImage.value ?? image).width());
              personSelection.next({
                focused: true,
                loading: false,
                people: present,
                selectedId: present.some(person => person.id === personSelection.value.selectedId)
                  ? personSelection.value.selectedId
                  : null,
                error: null,
              });
              // The faces are found after the people are shown, because the face models take a moment.
              setTimeout(() => {
                if (requestId === currentRequest && personSelection.value.focused) {
                  findFaceBoxes(image).catch(error => console.warn('[faces] could not find the faces:', errorMessage(error)));
                }
              }, 0);
            },
            error => {
              if (requestId !== currentRequest || !personSelection.value.focused) return;
              console.warn('[people] detection failed:', errorMessage(error));
              personSelection.next({
                ...personSelection.value,
                focused: true,
                loading: false,
                error: errorMessage(error),
              });
            },
          );
        }),
      // Only face tools need a close-up. Skin and other body tools still use person selection,
      // but leave the photo at its current scale.
      combineLatest([
        isFaceControlFocused$,
        personSelection$.pipe(
          // With one person there is nothing to pick, so that person is the one to zoom to. `focused` is part of the key so
          // that opening a face control again zooms in again after the person zoomed out by hand.
          map(state => [state.focused, state.selectedId ?? (state.people.length === 1 ? state.people[0].id : null)] as const),
          distinctUntilChanged(([previousFocused, previousId], [focused, id]) => previousFocused === focused && previousId === id),
        ),
        faceBoxes$,
      ])
        .pipe(
          map(([isFaceControl, [focused, selectedId], boxes]) =>
            [isFaceControl, selectedId, selectedId === null ? undefined : boxes?.[selectedId], focused] as const,
          ),
          distinctUntilChanged(
            ([previousFaceControl, previousId, previousFace, previousFocused], [isFaceControl, id, face, focused]) =>
              previousFaceControl === isFaceControl && previousId === id && previousFace === face && previousFocused === focused,
          ),
        )
        .subscribe(([isFaceControl, selectedId, face]) => {
          if (__DEV__) console.debug('[zoom] face tool zoom', isFaceControl, selectedId, !!face, 'focused:', personSelection.value.focused);
          const image = getPersonSelectionImage();
          const box = previewBox.value;
          if (!personSelection.value.focused || !image || !box) return;
          if (!isFaceControl) return springPreview({ x: 0, y: 0, scale: 1 });
          if (selectedId === null) return springPreview({ x: 0, y: 0, scale: 1 });
          const mask = people.value?.find(person => person.id === selectedId)?.mask;
          if (!face || !mask) return;

          // Where the face is on the contained photo, as an offset from the middle of the preview.
          const contain = Math.min(box.width / image.width(), box.height / image.height());
          const side = (face.width / mask.width()) * image.width() * contain;
          const centerX = ((face.x + face.width / 2) / mask.width() - 0.5) * image.width() * contain;
          const centerY = ((face.y + face.height / 2) / mask.height() - 0.5) * image.height() * contain;
          const scale = Math.min(MAX_FACE_ZOOM, Math.max(1, Math.min(box.width, box.height) / (side * FACE_FILL)));
          springPreview({ x: -centerX * scale, y: -centerY * scale, scale });
        }),
      previewPressed$.subscribe(event => {
        if (!personSelection.value.focused) return;
        const image = getPersonSelectionImage();
        const box = previewBox.value;
        if (!image || !box) return;

        const point = computePreviewImagePoint(
          image.width(),
          image.height(),
          box,
          previewScaleValue.value,
          previewTranslateValue.value,
          event,
          previewRotationValue.value,
        );
        if (!point) return;

        personOutlinesVisible.next(true);
        try {
          editorPreview.hide();
          const selectedId = personAt(point.x, point.y, image.width(), image.height());
          if (selectedId !== undefined && personSelection.value.people.some(person => person.id === selectedId)) personSelection.next({ ...personSelection.value, selectedId });
        } catch (error) {
          personSelection.next({ ...personSelection.value, error: errorMessage(error) });
        }
      }),
    ];
    return () => {
      requestId++;
      subscriptions.forEach(subscription => subscription.unsubscribe());
    };
  }, []);

  return null;
}

/** Goes back to editing everyone in the photo; sits beside the undo and redo buttons. */
export function PersonEveryoneButton() {
  const state = useObservable(personSelection$, personSelection.value);
  const { colors } = useTheme();
  const styles = useMemo(() => createStyles(colors), [colors]);

  // One person (or none) leaves nothing to choose: the effect is for everyone in the photo.
  if (!selectionNeeded(state)) return null;

  return (
    <Pressable
      onPress={() => {
        const image = getPersonSelectionImage();
        if (!image) return;
        try {
          editorPreview.hide();
          personSelection.next({ ...personSelection.value, selectedId: null, error: null });
        } catch (error) {
          personSelection.next({ ...personSelection.value, error: errorMessage(error) });
        }
      }}
      style={[styles.chip, state.selectedId === null && styles.disabled]}
      disabled={state.selectedId === null}
    >
      <Text style={styles.label}>Everyone</Text>
    </Pressable>
  );
}

function getPersonSelectionImage() {
  return editBaseImage.value ?? currentImage.value;
}

function createStyles(colors: ReturnType<typeof useTheme>['colors']) {
  return StyleSheet.create({
    chip: { paddingHorizontal: 12, paddingVertical: 6, borderRadius: 16, backgroundColor: colors.chip },
    disabled: { opacity: 0.45 },
    label: { color: colors.textPrimary, fontSize: 13, fontWeight: '600' },
  });
}
