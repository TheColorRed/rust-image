import type { Handles, ImagePreview } from '@alakazam/mobile';
import { VesselView } from '@vessel/react-native';
import { useEffect, useMemo, useRef } from 'react';
import { PanResponder, PixelRatio, StyleSheet, View } from 'react-native';
import type { PreviewBox } from '@/src/state/gestures';

/**
 * Handles laid over the photo: the points of a tool (a gradient's two ends, a perspective correction's four corners, a straightening line's two points)
 * that a finger grabs and moves. The handles, their lines, keeping them on the same places of the photo as it is zoomed, panned and turned, and what a
 * touch on one does are the native `Handles`; this is the view they are drawn on and the one place that decides whose a touch is. A finger that lands on a
 * handle belongs to the handles. Any other touch goes to the preview, so the photo can still be pinched and panned, and a second finger always does,
 * so a pinch that starts on a handle is still a pinch.
 *
 * The tool gives `handles` (made with its own number of points, shapes and joining) and the preview they follow, and hears `onMove` with the points
 * as a handle moves, to show its result, and `onEnd` when the finger lifts or another gesture takes over, to save it. The points are shares of the photo,
 * as `Handles.points()` has them.
 */
export function HandlesOverlay({
  handles,
  view,
  box,
  onMove,
  onEnd,
}: {
  handles: Handles;
  /** The preview the handles follow: the one that shows the photo as the tool sees it. */
  view: ImagePreview | null;
  /** The preview's size in dp. */
  box: PreviewBox;
  onMove: (points: number[]) => void;
  /** `moved` is whether a handle moved at all, so a tap that moves nothing saves nothing. */
  onEnd: (points: number[], moved: boolean) => void;
}) {
  const density = PixelRatio.get();
  // Reading the current callbacks through a ref keeps the responder alive as long as the handles are, whatever the tool does between renders.
  const latest = useRef({ onMove, onEnd });
  latest.current = { onMove, onEnd };

  useEffect(() => {
    if (view) handles.follow(view);
  }, [handles, view]);

  const responder = useMemo(() => {
    // Where the finger landed, in the preview's pixels. The handles move by how far it goes from there.
    let from = { x: 0, y: 0 };
    let moved = false;
    const finish = () => {
      if (handles.held() < 0) return;
      handles.endDrag();
      const wasMoved = moved;
      moved = false;
      latest.current.onEnd(handles.points(), wasMoved);
    };
    return PanResponder.create({
      // Only a touch on a handle is the handles'; the rest is left to the preview.
      onStartShouldSetPanResponder: event => handles.handleAt(event.nativeEvent.locationX * density, event.nativeEvent.locationY * density) >= 0,
      // A second finger is a pinch, which belongs to the preview.
      onPanResponderTerminationRequest: event => event.nativeEvent.touches.length > 1,
      onPanResponderGrant: event => {
        moved = false;
        from = { x: event.nativeEvent.locationX * density, y: event.nativeEvent.locationY * density };
        handles.grab(from.x, from.y);
      },
      onPanResponderMove: (_event, gesture) => {
        if (handles.held() < 0) return;
        moved = true;
        handles.dragTo(from.x + gesture.dx * density, from.y + gesture.dy * density);
        latest.current.onMove(handles.points());
      },
      onPanResponderRelease: finish,
      // Another responder took the touch (a pinch): what was moved so far is kept.
      onPanResponderTerminate: finish,
    });
  }, [handles, density]);

  if (!VesselView) return null;
  // The native view reaches past the preview on every side so a handle at its edge is whole, and the layer clips it to the preview.
  const pad = handles.overscan() / density;
  return (
    <View pointerEvents="box-none" style={[StyleSheet.absoluteFill, styles.layer]}>
      <VesselView
        source={handles}
        pointerEvents="none"
        style={{ position: 'absolute', left: -pad, top: -pad, width: box.width + pad * 2, height: box.height + pad * 2 }}
      />
      <View {...responder.panHandlers} style={StyleSheet.absoluteFill} />
    </View>
  );
}

const styles = StyleSheet.create({
  layer: { overflow: 'hidden' },
});
