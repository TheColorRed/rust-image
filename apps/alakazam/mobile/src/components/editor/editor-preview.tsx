import { BlemishReticle } from '@/src/components/tools/blemish-tool';
import { comparisonImage, shownImage } from '@/src/lib/editor/editor-pictures';
import { editorPreview } from '@/src/lib/editor/editor-preview';
import { useTheme, type ThemeColors } from '@/src/lib/theme';
import {
  restorePreviewRequested,
  restorePreviewRequested$,
  showCheckpointRequested,
  showCheckpointRequested$,
} from '@/src/state/commands';
import { isBlemishToolFocused$, isPersonControlFocused$ } from '@/src/state/edits';
import {
  ensurePreviewPanResponder,
  previewBox,
  previewBox$,
  previewLayoutFrame,
  previewOrigin,
  previewPanResponder$,
  previewPressed,
  previewScale,
  previewScale$,
  previewScaleValue,
  previewScaleSettled$,
  previewTranslateX,
  previewTranslateValue,
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
import {
  busy$,
  currentImage,
  currentImage$,
  editBaseImage,
  editBaseImage$,
  loadError as previewError,
  loadError$,
} from '@/src/state/session';
import { faceBoxes, faceBoxes$ } from '@/src/state/analysis';
import { personOutlinesVisible$, personSelection, personSelection$, selectionNeeded } from '@/src/state/person-selection';
import { ImagePreview, type AbraImage } from '@alakazam/mobile';
import { VesselView } from '@vessel/react-native';
import Svg, { Path } from 'react-native-svg';
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
  // Set when a touch ended a hold, whose release is not a tap on the photo.
  const holdEnded = useRef(false);
  const touchTargetRef = useRef<ElementRef<typeof Pressable> | null>(null);
  const [baseView, setBaseView] = useState<ImagePreview | null>(null);
  // The base preview that is alive now. State is a render behind it, and a viewport sent to one that was just let go of fails.
  const baseViewRef = useRef<ImagePreview | null>(null);
  const liveView = useSyncObservable(editorPreview.view, null);
  const box = useObservable(previewBox$, null);
  const loadError = useObservable(loadError$, null);
  const busy = useObservable(busy$, false);
  const blemishFocused = useObservable(isBlemishToolFocused$, false);
  const skinControlFocused = useObservable(isPersonControlFocused$, false);
  const personSelectionState = useObservable(personSelection$, personSelection.value);
  // People are found in the edit base, not in the screen-size preview, so their boxes are in its pixels.
  const editBase = useObservable(editBaseImage$, editBaseImage.value);
  const openImage = useObservable(currentImage$, currentImage.value);
  const detectedImage = editBase ?? openImage;
  const outlinesVisible = useSyncObservable(personOutlinesVisible$, false);
  const scale = useObservable(previewScale$, previewScale.value);
  const translateX = useObservable(previewTranslateX$, previewTranslateX.value);
  const translateY = useObservable(previewTranslateY$, previewTranslateY.value);
  const panResponder = useObservable(previewPanResponder$, null);
  // The native views always fill the preview box. Zoom and pan only move the camera: the part of the photo the box shows
  // is sent to the GPU on every change, which draws it from the photo that already sits there. Nothing is laid out again,
  // so the photo is complete at every frame of a gesture, and the part past its edges is black.
  useEffect(() => {
    if (!photo || !box) return;
    const fit = Math.min(box.width / photo.width, box.height / photo.height);
    const size = { width: Math.max(1, photo.width * fit), height: Math.max(1, photo.height * fit) };
    const subscription = combineLatest([previewScaleValue, previewTranslateValue]).subscribe(([zoom, move]) => {
      const left = (-box.width / 2 - move.x) / zoom;
      const top = (-box.height / 2 - move.y) / zoom;
      const viewport: [number, number, number, number] = [
        (left + size.width / 2) / size.width,
        (top + size.height / 2) / size.height,
        box.width / zoom / size.width,
        box.height / zoom / size.height,
      ];
      baseViewRef.current?.setViewport(...viewport);
      // The live preview that is open now, not `liveView` from the render: another effect in the same commit may have replaced it
      // (`ensure`) and let go of that one, and `release` empties `view` before it does.
      editorPreview.view.value?.setViewport(...viewport);
    });
    return () => subscription.unsubscribe();
  }, [photo, box, baseView, liveView]);
  const { colors } = useTheme();
  const styles = useMemo(() => createStyles(colors), [colors]);

  const beginCheckpointHold = useCallback(() => {
    // Re-measure on every touch-down: a layout-time measurement can be stale (screen transition,
    // insets settling), which showed up as the zoom drifting vertically.
    touchTargetRef.current?.measureInWindow((left, top) => previewOrigin.next({ x: left, y: top }));
    if (checkpointHoldTimer.current) clearTimeout(checkpointHoldTimer.current);
    checkpointPreviewActive.current = false;
    holdEnded.current = false;
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
    holdEnded.current = true;
    restorePreviewRequested.next();
  }, []);

  const handlePreviewPress = useCallback((event: GestureResponderEvent) => {
    if (holdEnded.current) return;
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
        touchTargetRef.current?.measureInWindow((left, top) => previewOrigin.next({ x: left, y: top }));
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

  // Show the source image on the native view under the live one; Vessel draws it at the view's size. One preview is open over
  // the edit base for as long as it is the same photo. It shares an image of its own, and each new source is put in that
  // image, so the photo stays on the GPU and a picture that is only the photo with effects over it costs the effects. A new
  // photo opens a new preview.
  useEffect(() => {
    let current: ImagePreview | null = null;
    let display: AbraImage | null = null;
    let over: unknown = null;
    const release = () => {
      baseViewRef.current = null;
      current?.uniffiDestroy();
      display?.uniffiDestroy();
      current = null;
      display = null;
      over = null;
      setBaseView(null);
    };
    const subscription = combineLatest([previewSourceImage$, previewBox$, editBaseImage$]).subscribe(([source, size, base]) => {
      if (!source || !size || !base) return release();
      if (over !== base || !current || !display) {
        release();
        display = source.clone() as AbraImage;
        current = new ImagePreview(display);
        over = base;
        baseViewRef.current = current;
        setBaseView(current);
      } else {
        display.replaceWith(source);
        current.refresh();
      }
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
    // The layout style carries the live view's opacity, so every new layout needs it applied again.
  }, [photo, box]);

  // A long press is a temporary before/after preview that restores on release; it changes no edit state.
  useEffect(() => {
    const subscriptions = [
      showCheckpointRequested$.subscribe(() => {
        showingCheckpoint.next(true);
        previewSourceImage.next(comparisonImage() ?? shownImage());
      }),
      restorePreviewRequested$.subscribe(() => {
        if (!showingCheckpoint.value) return;
        showingCheckpoint.next(false);
        previewSourceImage.next(shownImage());
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
          <>
            <VesselView
              source={baseView}
              pointerEvents="none"
              style={{ position: 'absolute', left: 0, top: 0, width: box.width, height: box.height, opacity: 1 }}
            />
            <VesselView
              ref={liveViewRef}
              source={liveView}
              pointerEvents="none"
              style={{ position: 'absolute', left: 0, top: 0, width: box.width, height: box.height, opacity: 0 }}
            />
          </>
        )}
        {photo && box && VesselView && (
          <Animated.View
            pointerEvents="none"
            style={{
              position: 'absolute',
              width: box.width,
              height: box.height,
              transform: [{ translateX }, { translateY }, { scale }],
            }}
          >
            {skinControlFocused && personSelectionState.focused && outlinesVisible && selectionNeeded(personSelectionState) && (
              <PersonDetectionOverlay
                people={personSelectionState.people}
                selectedId={personSelectionState.selectedId}
                loading={personSelectionState.loading}
                error={personSelectionState.error}
                imageSize={detectedImage ? { width: detectedImage.width(), height: detectedImage.height() } : photo}
                previewSize={box}
              />
            )}
          </Animated.View>
        )}
        {photo && box && VesselView && skinControlFocused && personSelectionState.focused && outlinesVisible && selectionNeeded(personSelectionState) && (
          // Outside the zoomed view so the notice keeps its size and stays centered while the photo is zoomed.
          <View
            pointerEvents="none"
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
              {personSelectionState.error
                ? personSelectionState.error
                : personSelectionState.loading
                  ? 'Finding bodies…'
                  : personSelectionState.people.length === 0
                    ? 'No bodies detected; skin effects still work'
                    : personSelectionState.selectedId === null
                      ? 'Tap a person to target skin effects'
                      : `Body ${personSelectionState.selectedId + 1} selected`}
            </Text>
          </View>
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
  const faces = useObservable(faceBoxes$, faceBoxes.value);
  const scale = useObservable(previewScaleSettled$, previewScaleValue.value);
  const fit = Math.min(previewSize.width / imageSize.width, previewSize.height / imageSize.height);
  const offsetX = (previewSize.width - imageSize.width * fit) / 2;
  const offsetY = (previewSize.height - imageSize.height * fit) / 2;

  return (
    <View pointerEvents="none" style={StyleSheet.absoluteFill}>
      {people.map(person => {
        const selected = person.id === selectedId;
        const face = faces?.[person.id];
        const color = PERSON_OUTLINE_COLORS[person.id % PERSON_OUTLINE_COLORS.length];
        const opacity = selectedId === null || selected ? 1 : 0.55;
        // The overlay is zoomed with the photo, so divide by the zoom to keep the lines the same thickness on screen.
        const strokeWidth = (selected ? 3 : 2) / scale;
        // A face is framed by a short arc at each side of a tall oval, with the top and bottom left open, and never filled in.
        if (face) {
          const cx = offsetX + (face.x + face.width / 2) * fit;
          const cy = offsetY + (face.y + face.height / 2) * fit;
          const rx = face.width * fit * 0.47;
          const ry = face.height * fit * 0.62;
          const point = (degrees: number) => {
            const radians = (degrees * Math.PI) / 180;
            return `${cx + rx * Math.cos(radians)} ${cy + ry * Math.sin(radians)}`;
          };
          const arc = (from: number, to: number) => `M ${point(from)} A ${rx} ${ry} 0 0 1 ${point(to)}`;
          return (
            <Svg key={person.id} style={StyleSheet.absoluteFill} opacity={opacity}>
              <Path d={arc(-28, 28)} stroke={color} strokeWidth={strokeWidth} strokeLinecap="round" fill="none" />
              <Path d={arc(152, 208)} stroke={color} strokeWidth={strokeWidth} strokeLinecap="round" fill="none" />
            </Svg>
          );
        }
        // Until the face is found, the frame is the box around the body.
        return (
          <View
            key={person.id}
            style={{
              position: 'absolute',
              left: offsetX + person.x * fit,
              top: offsetY + person.y * fit,
              width: person.width * fit,
              height: person.height * fit,
              opacity,
              borderWidth: strokeWidth,
              borderColor: color,
            }}
          />
        );
      })}
    </View>
  );
}

function createStyles(colors: ThemeColors) {
  return StyleSheet.create({
    area: { flex: 1, alignItems: 'center', justifyContent: 'center' },
    touchTarget: { flex: 1, alignSelf: 'stretch', alignItems: 'center', justifyContent: 'center' },
    busy: { position: 'absolute', bottom: 12, alignSelf: 'center' },
    error: { color: colors.danger, paddingHorizontal: 24, textAlign: 'center' },
  });
}
