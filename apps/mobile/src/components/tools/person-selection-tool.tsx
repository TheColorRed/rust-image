import { useEffect, useMemo } from 'react';
import { Pressable, StyleSheet, Text, View } from 'react-native';
import { useObservable } from 'react-rx';
import { combineLatest } from 'rxjs';
import { distinctUntilChanged, map } from 'rxjs/operators';
import { computePreviewImagePoint } from '@/src/lib/preview-coordinates';
import { editorLive } from '@/src/lib/editor-live';
import { useTheme } from '@/src/lib/theme';
import { isSkinControlFocused$ } from '@/src/state/edits';
import { personOutlinesVisible, personSelection, personSelection$ } from '@/src/state/person-selection';
import { previewBox, previewPressed$, previewScaleValue, previewTranslateValue } from '@/src/state/gestures';
import { currentImage, currentImage$, editBaseImage, editBaseImage$ } from '@/src/state/session';

/** Detects people while the selector is open and maps image taps to the Rust-owned instance masks. */
export function PersonSelectionToolController() {
  useEffect(() => {
    let requestId = 0;
    const subscriptions = [
      combineLatest([isSkinControlFocused$, currentImage$, editBaseImage$])
        .pipe(
          map(([isOpen, image, editBase]) => [isOpen, editBase ?? image] as const),
          distinctUntilChanged(
            ([previousOpen, previousImage], [isOpen, image]) => previousOpen === isOpen && previousImage === image,
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
          personSelection.next({
            ...personSelection.value,
            focused: true,
            loading: true,
            error: null,
          });
          image.findPeopleAsync().then(
            people => {
              if (requestId !== currentRequest || !personSelection.value.focused) return;
              personSelection.next({
                focused: true,
                loading: false,
                people,
                selectedId: image.selectedPerson() ?? null,
                error: null,
              });
            },
            error => {
              if (requestId !== currentRequest || !personSelection.value.focused) return;
              personSelection.next({
                ...personSelection.value,
                focused: true,
                loading: false,
                error: String((error as Error)?.message ?? error),
              });
            },
          );
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
        );
        if (!point) return;

        personOutlinesVisible.next(true);
        try {
          editorLive.hide();
          const selectedId = image.selectPersonAt(point.x, point.y);
          if (selectedId !== undefined) personSelection.next({ ...personSelection.value, selectedId });
        } catch (error) {
          personSelection.next({ ...personSelection.value, error: String((error as Error)?.message ?? error) });
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

/** Controls the selected person alongside the skin slider without replacing it. */
export function PersonSelectionControls() {
  const state = useObservable(personSelection$, personSelection.value);
  const { colors } = useTheme();
  const styles = useMemo(() => createStyles(colors), [colors]);

  return (
    <View style={styles.row}>
      <Text style={styles.status} numberOfLines={2}>
        {state.error ??
          (state.loading
            ? 'Finding people…'
            : state.people.length === 0
              ? 'No people found; skin effects still work on the image'
              : state.selectedId === null
                ? `Tap a person (${state.people.length} found)`
                : `Person ${state.selectedId + 1} selected`)}
      </Text>
      <Pressable
        onPress={() => {
          const image = getPersonSelectionImage();
          if (!image) return;
          try {
            editorLive.hide();
            image.selectPerson(undefined);
            personSelection.next({ ...personSelection.value, selectedId: null, error: null });
          } catch (error) {
            personSelection.next({ ...personSelection.value, error: String((error as Error)?.message ?? error) });
          }
        }}
        style={[styles.chip, state.selectedId === null && styles.disabled]}
        disabled={state.selectedId === null}
      >
        <Text style={styles.label}>All people</Text>
      </Pressable>
    </View>
  );
}

function getPersonSelectionImage() {
  return editBaseImage.value ?? currentImage.value;
}

function createStyles(colors: ReturnType<typeof useTheme>['colors']) {
  return StyleSheet.create({
    row: { flexDirection: 'row', alignItems: 'center', paddingHorizontal: 12, gap: 10 },
    chip: { paddingHorizontal: 12, paddingVertical: 8, borderRadius: 16, backgroundColor: colors.chip },
    disabled: { opacity: 0.45 },
    label: { color: colors.textPrimary, fontSize: 13, fontWeight: '600' },
    status: { flex: 1, color: colors.textSecondary, fontSize: 12, textAlign: 'center' },
  });
}
