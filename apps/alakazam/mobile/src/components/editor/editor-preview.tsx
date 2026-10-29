// @refresh reset
import { PersonDetectionNotice, PersonDetectionOverlay, personOverlayShown } from '@/src/components/editor/person-detection-overlay';
import { ViewControls } from '@/src/components/editor/view-controls';
import { BlemishReticle } from '@/src/components/tools/blemish-tool';
import { AreaOutlines } from '@/src/components/tools/color-area-tool';
import { DrawAreaOverlay } from '@/src/components/tools/draw-area';
import { areaDrawKey$, areaDrawMode, areaTarget$ } from '@/src/state/draw-area';
import { CropGizmos } from '@/src/components/tools/crop-tool';
import { useBasePreview } from '@/src/hooks/useBasePreview';
import { usePreviewBox } from '@/src/hooks/usePreviewBox';
import { usePreviewCamera } from '@/src/hooks/usePreviewCamera';
import { usePreviewHold } from '@/src/hooks/usePreviewHold';
import { comparisonImage, shownImage } from '@/src/lib/editor/editor-pictures';
import { editorPreview } from '@/src/lib/editor/editor-preview';
import { useTheme, type ThemeColors } from '@/src/lib/ui/theme';
import { restorePreviewRequested$, showCheckpointRequested$ } from '@/src/state/commands';
import { isBlemishToolFocused$, isCropToolFocused$, isPersonControlFocused$ } from '@/src/state/edits';
import {
  ensurePreviewPanResponder,
  previewBox$,
  previewMoving,
  previewPanResponder$,
  previewRotation$,
  previewRotation,
  previewScale$,
  previewScale,
  previewTranslateX$,
  previewTranslateX,
  previewTranslateY$,
  previewTranslateY,
  resetPreviewTransform,
  startPreviewGestures,
  trackPreviewTransform,
} from '@/src/state/gestures';
import { liveSurface$, previewSize$, previewSourceImage, showingCheckpoint } from '@/src/state/preview';
import { busy$, currentImage, currentImage$, editBaseImage, editBaseImage$, loadError$ } from '@/src/state/session';
import { personOutlinesVisible$, personSelection, personSelection$ } from '@/src/state/person-selection';
import { Gizmos } from '@alakazam/mobile';
import { editorGizmos, editorGizmos$ } from '@/src/state/gizmos';
import { useOwning } from '@/src/lib/native/native-handles';
import { VesselView } from '@vessel/react-native';
import { useEffect, useMemo, useRef, type ElementRef } from 'react';
import { ActivityIndicator, Animated, Pressable, StyleSheet, Text, View } from 'react-native';
import { useObservable, useSyncObservable } from 'react-rx';

/**
 * The photo itself: renders `previewSourceImage` at the preview box's size, owns pinch/pan, the
 * long-press before/after comparison, and the busy/error/saved overlays.
 */
export function EditorPreview() {
  const photo = useObservable(previewSize$, null);
  // The size of the frames on screen while a preview is open. A tool's result can have another shape than the photo.
  const liveShown = useObservable(liveSurface$, null);
  const liveViewRef = useRef<ElementRef<NonNullable<typeof VesselView>> | null>(null);
  const touchTargetRef = useRef<ElementRef<typeof Pressable> | null>(null);
  const liveView = useSyncObservable(editorPreview.view, null);
  const box = useObservable(previewBox$, null);
  const gizmos = useObservable(editorGizmos$, null);
  const loadError = useObservable(loadError$, null);
  const busy = useObservable(busy$, false);
  const blemishFocused = useObservable(isBlemishToolFocused$, false);
  const drawingArea = useObservable(areaDrawKey$, null);
  const areaTarget = useObservable(areaTarget$, null);
  const cropFocused = useObservable(isCropToolFocused$, false);
  const skinControlFocused = useObservable(isPersonControlFocused$, false);
  const personSelectionState = useObservable(personSelection$, personSelection.value);
  // People are found in the edit base, not in the screen-size preview, so their boxes are in its pixels.
  const editBase = useObservable(editBaseImage$, editBaseImage.value);
  const openImage = useObservable(currentImage$, currentImage.value);
  const detectedImage = editBase ?? openImage;
  const outlinesVisible = useSyncObservable(personOutlinesVisible$, false);
  const scale = useObservable(previewScale$, previewScale.value);
  const rotation = useObservable(previewRotation$, previewRotation.value);
  const translateX = useObservable(previewTranslateX$, previewTranslateX.value);
  const translateY = useObservable(previewTranslateY$, previewTranslateY.value);
  const panResponder = useObservable(previewPanResponder$, null);
  const { colors } = useTheme();
  const styles = useMemo(() => createStyles(colors), [colors]);

  const { view: baseView, ref: baseViewRef } = useBasePreview();
  usePreviewCamera({ photo, liveShown, box, baseView, liveView, baseViewRef });
  const hold = usePreviewHold(touchTargetRef);
  const handleLayout = usePreviewBox(touchTargetRef);

  useEffect(() => {
    ensurePreviewPanResponder();
    const tracking = trackPreviewTransform();
    tracking.add(startPreviewGestures());
    return () => {
      tracking.unsubscribe();
      resetPreviewTransform();
      previewMoving.next(false);
    };
  }, []);

  // The gizmos over the photo: made once, shared by every control that draws on the preview (see `state/gizmos.ts`).
  const made = useOwning(keep => keep(new Gizmos()), []);
  useEffect(() => {
    editorGizmos.next(made);
    return () => editorGizmos.next(null);
  }, [made]);

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

  const showPeople = personOverlayShown(personSelectionState, skinControlFocused, outlinesVisible);

  return (
    <View
      style={[styles.area, { marginTop: 56 }]}
      onLayout={handleLayout}
      {...(panResponder?.panHandlers ?? {})}
    >
      <Pressable
        ref={touchTargetRef}
        style={styles.touchTarget}
        onPressIn={hold.onPressIn}
        onPressOut={hold.onPressOut}
        onTouchMove={hold.onTouchMove}
        onPress={hold.onPress}
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
              // The photo turns about the middle of the box after it is zoomed and panned, so the turn is the outermost.
              transform: [
                { rotate: rotation.interpolate({ inputRange: [0, 1], outputRange: ['0rad', '1rad'] }) },
                { translateX },
                { translateY },
                { scale },
              ],
            }}
          >
            {showPeople && (
              <PersonDetectionOverlay
                people={personSelectionState.people}
                selectedId={personSelectionState.selectedId}
                imageSize={detectedImage ? { width: detectedImage.width(), height: detectedImage.height() } : photo}
                previewSize={box}
              />
            )}
            <AreaOutlines imageSize={photo} previewSize={box} />
          </Animated.View>
        )}
        {photo && box && VesselView && showPeople && <PersonDetectionNotice state={personSelectionState} />}
        {gizmos && box && VesselView && (
          <VesselView
            source={gizmos}
            pointerEvents="none"
            style={{ position: 'absolute', left: 0, top: 0, width: box.width, height: box.height }}
          />
        )}
        {blemishFocused && <BlemishReticle />}
        {areaTarget && drawingArea === areaTarget.key ? (
          <DrawAreaOverlay
            gizmo={areaTarget.key}
            opacity={areaTarget.opacity}
            minShare={areaTarget.minShare}
            onDrawn={loop => {
              const mode = areaDrawMode.value;
              areaTarget.onDrawn(loop, mode);
              areaTarget.onChange?.({ type: 'draw', loop, mode });
            }}
          />
        ) : null}
        {cropFocused && <CropGizmos />}
        <ViewControls />
        {busy && <ActivityIndicator style={styles.busy} color={colors.textPrimary} />}
      </Pressable>
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
