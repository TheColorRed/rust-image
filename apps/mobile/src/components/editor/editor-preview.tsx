import { AlphaType, Canvas, ColorType, Skia, type SkImage, Image as SkiaImage } from '@shopify/react-native-skia';
import { useCallback, useEffect, useMemo } from 'react';
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
  preview,
  liveFrame,
  liveFrame$,
  liveSurface$,
  preview$,
  previewSourceImage,
  previewSourceImage$,
  showingCheckpoint,
  topInset$,
} from '@/src/state/preview';
import { busy$, currentImage, loadError$, saved, saved$ } from '@/src/state/session';
import { AbraLiveView, LIVE_SURFACE_ID } from '@/src/components/editor/abra-live-view';
import { BlemishReticle } from '@/src/components/tools/blemish-tool';

/**
 * How long a replaced preview `SkImage` lingers before it's disposed. Skia's Canvas can still be
 * drawing the old image a frame or two after the new one is set; touching a disposed image throws
 * "Attempted to access a disposed object", which leaves Skia's reconciler stuck and surfaces as
 * "Should not already be working" on every later render.
 */
const DISPOSE_DELAY_MS = 1000;

/** How long the "Edits applied" toast stays up; it has no dismiss control of its own. */
const SAVED_TOAST_MS = 2000;

/**
 * The photo itself: renders `previewSourceImage` at the preview box's size, owns pinch/pan, the
 * long-press before/after comparison, and the busy/error/saved overlays.
 */
export function EditorPreview() {
  const topInset = useObservable(topInset$, 0);
  const committed = useObservable(preview$, null);
  const live = useObservable(liveFrame$, null);
  const image = live ?? committed;
  const surface = useObservable(liveSurface$, null);
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

  // Rasterize the source image at the preview's pixel size. The replaced SkImage is disposed
  // later (see DISPOSE_DELAY_MS) so a still-mounted Canvas can't draw it after disposal.
  useEffect(() => {
    const subscription = combineLatest([previewSourceImage$, previewBox$]).subscribe(([source, size]) => {
      if (!source || !size) return;
      const ratio = PixelRatio.get();
      const pixels = source.preview(Math.round(size.width * ratio), Math.round(size.height * ratio));
      const stale = preview.value;
      liveFrame.next(null);
      preview.next(
        Skia.Image.MakeImage(
          { width: pixels.width, height: pixels.height, colorType: ColorType.RGBA_8888, alphaType: AlphaType.Unpremul },
          Skia.Data.fromBytes(new Uint8Array(pixels.data)),
          pixels.width * 4,
        ),
      );
      if (stale) setTimeout(() => stale.dispose(), DISPOSE_DELAY_MS);
    });
    return () => {
      subscription.unsubscribe();
      const stale = preview.value;
      preview.next(null);
      if (stale) setTimeout(() => stale.dispose(), DISPOSE_DELAY_MS);
    };
  }, []);

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
        {!loadError && !image && <ActivityIndicator size="large" color={colors.textPrimary} />}
        {image && box && (
          <Animated.View
            style={{ width: box.width, height: box.height, transform: [{ translateX }, { translateY }, { scale }] }}
          >
            <Canvas style={{ width: box.width, height: box.height }}>
              <SkiaImage image={image} x={0} y={0} width={box.width} height={box.height} fit="contain" />
            </Canvas>
            {AbraLiveView && (
              <AbraLiveView
                surfaceId={LIVE_SURFACE_ID}
                pointerEvents="none"
                style={liveSurfaceStyle(box, committed, surface !== null)}
              />
            )}
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
 * Fits the photo's aspect ratio inside the preview box, like the Canvas's `contain`. The view keeps this size while
 * hidden so its surface is already the right size when the first frame is drawn on it.
 */
function liveSurfaceStyle(p_box: { width: number; height: number }, p_photo: SkImage | null, p_visible: boolean) {
  const photo = p_photo ?? { width: () => p_box.width, height: () => p_box.height };
  const fit = Math.min(p_box.width / photo.width(), p_box.height / photo.height());
  const width = Math.max(1, photo.width() * fit);
  const height = Math.max(1, photo.height() * fit);
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
