import { ImagePreview } from '@alakazam/mobile';
import { useCallback, useEffect, useMemo, useRef, useState, type ElementRef } from 'react';
import {
  ActivityIndicator,
  Animated,
  PixelRatio,
  Pressable,
  StyleSheet,
  Text,
  View,
  type LayoutChangeEvent,
} from 'react-native';
import { useObservable } from 'react-rx';
import { combineLatest } from 'rxjs';
import { comparisonImage } from '@/src/lib/editor-history';
import { useTheme, type ThemeColors } from '@/src/lib/theme';
import {
  restorePreviewRequested,
  restorePreviewRequested$,
  showCheckpointRequested,
  showCheckpointRequested$,
} from '@/src/state/commands';
import { isBlemishToolFocused$ } from '@/src/state/edits';
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
import { busy$, currentImage, loadError$, saved, saved$ } from '@/src/state/session';
import { VesselView } from '@vessel/react-native';
import { editorLive } from '@/src/lib/editor-live';
import { BlemishReticle } from '@/src/components/tools/blemish-tool';

/** How long the "Edits applied" toast stays up; it has no dismiss control of its own. */
const SAVED_TOAST_MS = 2000;

/**
 * The photo itself: renders `previewSourceImage` at the preview box's size, owns pinch/pan, the
 * long-press before/after comparison, and the busy/error/saved overlays.
 */
export function EditorPreview() {
  const topInset = useObservable(topInset$, 0);
  const photo = useObservable(previewSize$, null);
  const liveViewRef = useRef<ElementRef<NonNullable<typeof VesselView>> | null>(null);
  const [baseView, setBaseView] = useState<ImagePreview | null>(null);
  const liveView = useObservable(editorLive.view, null);
  const box = useObservable(previewBox$, null);
  const loadError = useObservable(loadError$, null);
  const busy = useObservable(busy$, false);
  const isSaved = useObservable(saved$, false);
  const blemishFocused = useObservable(isBlemishToolFocused$, false);
  const scale = useObservable(previewScale$, previewScale.value);
  const translateX = useObservable(previewTranslateX$, previewTranslateX.value);
  const translateY = useObservable(previewTranslateY$, previewTranslateY.value);
  const panResponder = useObservable(previewPanResponder$, null);
  const { colors } = useTheme();
  const styles = useMemo(() => createStyles(colors), [colors]);

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
      resetPreviewTransform();
      if (previewLayoutFrame.value !== null) cancelAnimationFrame(previewLayoutFrame.value);
      previewLayoutFrame.next(null);
    };
  }, []);

  // Show the source image on the native view under the live one; Vessel draws it at the view's size. Each new source
  // replaces the previous preview, which lets go of the view as it goes.
  useEffect(() => {
    let current: ImagePreview | null = null;
    const release = () => {
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

  useEffect(() => {
    if (!isSaved) return;
    const timeout = setTimeout(() => saved.next(false), SAVED_TOAST_MS);
    return () => clearTimeout(timeout);
  }, [isSaved]);

  return (
    <View
      style={[styles.area, { marginTop: topInset + 56 }]}
      onLayout={handleLayout}
      {...(panResponder?.panHandlers ?? {})}
    >
      <Pressable
        style={styles.touchTarget}
        delayLongPress={250}
        onLongPress={() => showCheckpointRequested.next()}
        onPressOut={() => restorePreviewRequested.next()}
        onPress={event => previewPressed.next(event)}
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
          </Animated.View>
        )}
        {blemishFocused && <BlemishReticle />}
        {busy && <ActivityIndicator style={styles.busy} color={colors.textPrimary} />}
        {isSaved && (
          <View style={styles.toast} pointerEvents="none">
            <Text style={styles.toastText}>Edits applied</Text>
          </View>
        )}
      </Pressable>
    </View>
  );
}

/**
 * Fits the photo's aspect ratio inside the preview box, centered. The live view keeps this size while hidden so its
 * surface is already the right size when the first frame is drawn on it.
 */
function photoStyle(p_box: { width: number; height: number }, p_photo: { width: number; height: number }, p_visible: boolean) {
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
    toast: {
      position: 'absolute',
      bottom: 10,
      alignSelf: 'center',
      backgroundColor: 'rgba(0,0,0,0.75)',
      paddingHorizontal: 16,
      paddingVertical: 8,
      borderRadius: 20,
    },
    toastText: { color: '#fff', fontSize: 13, fontWeight: '600' },
  });
}
