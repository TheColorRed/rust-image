import { BlemishReticle } from '@/src/components/tools/blemish-tool';
import { comparisonImage } from '@/src/lib/editor-replay';
import { editorLive } from '@/src/lib/editor-live';
import { waitForPreviewFrame } from '@/src/lib/preview-handoff';
import { useTheme, type ThemeColors } from '@/src/lib/theme';
import {
  restorePreviewRequested,
  restorePreviewRequested$,
  showCheckpointRequested,
  showCheckpointRequested$,
} from '@/src/state/commands';
import { isBlemishToolFocused$, isSkinControlFocused$ } from '@/src/state/edits';
import {
  ensurePreviewPanResponder,
  previewBox,
  previewBox$,
  previewLayoutFrame,
  previewPanResponder$,
  previewPressed,
  previewScale,
  previewScale$,
  previewTranslateX,
  previewTranslateX$,
  previewTranslateY,
  previewTranslateY$,
  resetPreviewTransform,
  startPreviewGestures,
  trackPreviewTransform,
} from '@/src/state/gestures';
import {
  liveSurface$,
  previewSize,
  previewSize$,
  previewSourceImage,
  previewSourceImage$,
  showingCheckpoint,
  topInset$,
} from '@/src/state/preview';
import { busy$, currentImage, loadError as previewError, loadError$ } from '@/src/state/session';
import { personOutlinesVisible$, personSelection, personSelection$ } from '@/src/state/person-selection';
import { ImagePreview } from '@alakazam/mobile';
import { VesselView } from '@vessel/react-native';
import { useCallback, useEffect, useMemo, useRef, useState, type ElementRef } from 'react';
import {
  ActivityIndicator,
  Animated,
  Pressable,
  StyleSheet,
  Text,
  View,
  type GestureResponderEvent,
  type LayoutChangeEvent,
} from 'react-native';
import { useObservable, useSyncObservable } from 'react-rx';
import { combineLatest } from 'rxjs';

const CHECKPOINT_HOLD_MS = 250;
const PERSON_OUTLINE_COLORS = [
  '#FF4FA3',
  '#FF453A',
  '#34C759',
  '#0A84FF',
  '#FFD60A',
  '#FF9F0A',
  '#BF5AF2',
  '#64D2FF',
  '#A8E66B',
  '#FF6B6B',
] as const;

/**
 * The photo itself: renders `previewSourceImage` at the preview box's size, owns pinch/pan, the
 * long-press before/after comparison, and the busy/error/saved overlays.
 */
export function EditorPreview() {
  const topInset = useObservable(topInset$, 0);
  const photo = useObservable(previewSize$, null);
  const liveViewRef = useRef<ElementRef<NonNullable<typeof VesselView>> | null>(null);
  const checkpointHoldTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const checkpointPreviewActive = useRef(false);
  const touchTargetRef = useRef<ElementRef<typeof Pressable> | null>(null);
  const [baseView, setBaseView] = useState<ImagePreview | null>(null);
  const liveView = useSyncObservable(editorLive.view, null);
  const box = useObservable(previewBox$, null);
  const loadError = useObservable(loadError$, null);
  const busy = useObservable(busy$, false);
  const blemishFocused = useObservable(isBlemishToolFocused$, false);
  const skinControlFocused = useObservable(isSkinControlFocused$, false);
  const personSelectionState = useObservable(personSelection$, personSelection.value);
  const outlinesVisible = useSyncObservable(personOutlinesVisible$, false);
  const scale = useObservable(previewScale$, previewScale.value);
  const translateX = useObservable(previewTranslateX$, previewTranslateX.value);
  const translateY = useObservable(previewTranslateY$, previewTranslateY.value);
  const panResponder = useObservable(previewPanResponder$, null);
  const { colors } = useTheme();
  const styles = useMemo(() => createStyles(colors), [colors]);

  const beginCheckpointHold = useCallback(() => {
    if (checkpointHoldTimer.current) clearTimeout(checkpointHoldTimer.current);
    checkpointPreviewActive.current = false;
    checkpointHoldTimer.current = setTimeout(() => {
      checkpointHoldTimer.current = null;
      checkpointPreviewActive.current = true;
      showCheckpointRequested.next();
    }, CHECKPOINT_HOLD_MS);
  }, []);

  const endCheckpointHold = useCallback(() => {
    if (checkpointHoldTimer.current) clearTimeout(checkpointHoldTimer.current);
    checkpointHoldTimer.current = null;
    if (!checkpointPreviewActive.current) return;
    checkpointPreviewActive.current = false;
    restorePreviewRequested.next();
  }, []);

  const handlePreviewPress = useCallback((event: GestureResponderEvent) => {
    const { pageX, pageY } = event.nativeEvent;
    // locationX/Y can be relative to the transformed photo or outline child that was hit.
    touchTargetRef.current?.measureInWindow((left, top) => {
      previewPressed.next({ x: pageX - left, y: pageY - top, pageX, pageY });
    });
  }, []);

  // Debounced to a frame: a layout that shifts while an edit replay is in flight would otherwise
  // land two rAF-deferred state updates in one native frame batch.
  const handleLayout = useCallback((event: LayoutChangeEvent) => {
    const { width, height } = event.nativeEvent.layout;
    if (previewLayoutFrame.value !== null) cancelAnimationFrame(previewLayoutFrame.value);
    previewLayoutFrame.next(
      requestAnimationFrame(() => {
        previewLayoutFrame.next(null);
        const current = previewBox.value;
        if (current?.width === width && current.height === height) return;
        previewBox.next({ width, height });
      }),
    );
  }, []);

  useEffect(() => {
    ensurePreviewPanResponder();
    const tracking = trackPreviewTransform();
    tracking.add(startPreviewGestures());
    return () => {
      tracking.unsubscribe();
      if (checkpointHoldTimer.current) clearTimeout(checkpointHoldTimer.current);
      checkpointHoldTimer.current = null;
      if (checkpointPreviewActive.current) restorePreviewRequested.next();
      checkpointPreviewActive.current = false;
      resetPreviewTransform();
      if (previewLayoutFrame.value !== null) cancelAnimationFrame(previewLayoutFrame.value);
      previewLayoutFrame.next(null);
    };
  }, []);

  // Show the source image on the native view under the live one; Vessel draws it at the view's size. Each new source
  // replaces the previous preview, which lets go of the view as it goes.
  useEffect(() => {
    let current: ImagePreview | null = null;
    let cancelHandoff: (() => void) | undefined;
    const release = () => {
      cancelHandoff?.();
      cancelHandoff = undefined;
      current?.uniffiDestroy();
      current = null;
      setBaseView(null);
    };
    const subscription = combineLatest([previewSourceImage$, previewBox$]).subscribe(([source, size]) => {
      release();
      if (!source || !size) return;
      current = new ImagePreview(source);
      setBaseView(current);
      previewSize.next({ width: source.width(), height: source.height() });
      const activity = editorLive.activity;
      cancelHandoff = waitForPreviewFrame(
        current,
        () => {
          if (editorLive.activity === activity) editorLive.hide();
        },
        error => {
          console.warn('[preview] could not present the edited frame:', error);
          previewError.next(error.message);
        },
      );
    });
    return () => {
      subscription.unsubscribe();
      release();
      previewSize.next(null);
    };
  }, []);

  // The live view is shown and hidden straight on the native view. Going through React state took around 100 ms to reach
  // the screen, which is what tapping a mood felt like; a slider never paid it because its view stays visible while dragged.
  useEffect(() => {
    const subscription = liveSurface$.subscribe(size => {
      liveViewRef.current?.setNativeProps({ style: { opacity: size ? 1 : 0 } });
    });
    return () => subscription.unsubscribe();
    // Subscribing again once the view has mounted applies the current state to it.
  }, [photo, box]);

  // A long press is a temporary before/after preview that restores on release; it changes no edit state.
  useEffect(() => {
    const subscriptions = [
      showCheckpointRequested$.subscribe(() => {
        showingCheckpoint.next(true);
        previewSourceImage.next(comparisonImage() ?? currentImage.value);
      }),
      restorePreviewRequested$.subscribe(() => {
        if (!showingCheckpoint.value) return;
        showingCheckpoint.next(false);
        previewSourceImage.next(currentImage.value);
      }),
    ];
    return () => subscriptions.forEach(subscription => subscription.unsubscribe());
  }, []);

  return (
    <View
      style={[styles.area, { marginTop: topInset + 56 }]}
      onLayout={handleLayout}
      {...(panResponder?.panHandlers ?? {})}
    >
      <Pressable
        ref={touchTargetRef}
        style={styles.touchTarget}
        onPressIn={beginCheckpointHold}
        onPressOut={endCheckpointHold}
        onPress={handlePreviewPress}
      >
        {loadError && <Text style={styles.error}>{loadError}</Text>}
        {!loadError && !photo && <ActivityIndicator size="large" color={colors.textPrimary} />}
        {photo && box && VesselView && (
          <Animated.View
            style={{ width: box.width, height: box.height, transform: [{ translateX }, { translateY }, { scale }] }}
          >
            <VesselView source={baseView} pointerEvents="none" style={photoStyle(box, photo, true)} />
            <VesselView
              ref={liveViewRef}
              source={liveView}
              pointerEvents="none"
              style={photoStyle(box, photo, false)}
            />
            {skinControlFocused && personSelectionState.focused && outlinesVisible && (
              <PersonDetectionOverlay
                people={personSelectionState.people}
                selectedId={personSelectionState.selectedId}
                loading={personSelectionState.loading}
                error={personSelectionState.error}
                imageSize={photo}
                previewSize={box}
              />
            )}
          </Animated.View>
        )}
        {blemishFocused && <BlemishReticle />}
        {busy && <ActivityIndicator style={styles.busy} color={colors.textPrimary} />}
      </Pressable>
    </View>
  );
}

function PersonDetectionOverlay({
  people,
  selectedId,
  loading,
  error,
  imageSize,
  previewSize,
}: {
  people: typeof personSelection.value.people;
  selectedId: number | null;
  loading: boolean;
  error: string | null;
  imageSize: { width: number; height: number };
  previewSize: { width: number; height: number };
}) {
  const fit = Math.min(previewSize.width / imageSize.width, previewSize.height / imageSize.height);
  const offsetX = (previewSize.width - imageSize.width * fit) / 2;
  const offsetY = (previewSize.height - imageSize.height * fit) / 2;

  return (
    <View pointerEvents="none" style={StyleSheet.absoluteFill}>
      <View
        style={{
          position: 'absolute',
          top: 12,
          alignSelf: 'center',
          paddingHorizontal: 10,
          paddingVertical: 5,
          borderRadius: 14,
          backgroundColor: 'rgba(0,0,0,0.68)',
        }}
      >
        <Text style={{ color: '#fff', fontSize: 12, fontWeight: '600' }}>
          {error
            ? error
            : loading
              ? 'Finding bodies…'
              : people.length === 0
                ? 'No bodies detected; skin effects still work'
                : selectedId === null
                  ? 'Tap a body to target skin effects'
                  : `Body ${selectedId + 1} selected`}
        </Text>
      </View>
      {people.map(person => {
        const selected = person.id === selectedId;
        return (
          <View
            key={person.id}
            style={{
              position: 'absolute',
              left: offsetX + person.x * fit,
              top: offsetY + person.y * fit,
              width: person.width * fit,
              height: person.height * fit,
              borderWidth: selected ? 3 : 2,
              borderColor: PERSON_OUTLINE_COLORS[person.id % PERSON_OUTLINE_COLORS.length],
              backgroundColor: selected
                ? `${PERSON_OUTLINE_COLORS[person.id % PERSON_OUTLINE_COLORS.length]}22`
                : 'transparent',
            }}
          >
            <Text
              style={{
                position: 'absolute',
                top: -2,
                left: -2,
                paddingHorizontal: 5,
                paddingVertical: 2,
                overflow: 'hidden',
                color: '#fff',
                backgroundColor: PERSON_OUTLINE_COLORS[person.id % PERSON_OUTLINE_COLORS.length],
                fontSize: 11,
                fontWeight: '700',
              }}
            >
              {person.id + 1}
            </Text>
          </View>
        );
      })}
    </View>
  );
}

/**
 * Fits the photo's aspect ratio inside the preview box, centered. The live view keeps this size while hidden so its
 * surface is already the right size when the first frame is drawn on it.
 */
function photoStyle(
  p_box: { width: number; height: number },
  p_photo: { width: number; height: number },
  p_visible: boolean,
) {
  const fit = Math.min(p_box.width / p_photo.width, p_box.height / p_photo.height);
  const width = Math.max(1, p_photo.width * fit);
  const height = Math.max(1, p_photo.height * fit);
  return {
    position: 'absolute' as const,
    left: (p_box.width - width) / 2,
    top: (p_box.height - height) / 2,
    width,
    height,
    opacity: p_visible ? 1 : 0,
  };
}

function createStyles(colors: ThemeColors) {
  return StyleSheet.create({
    area: { flex: 1, alignItems: 'center', justifyContent: 'center' },
    touchTarget: { flex: 1, alignSelf: 'stretch', alignItems: 'center', justifyContent: 'center' },
    busy: { position: 'absolute', bottom: 12, alignSelf: 'center' },
    error: { color: colors.danger, paddingHorizontal: 24, textAlign: 'center' },
  });
}
