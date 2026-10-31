import { useEffect, useMemo } from 'react';
import { PanResponder, StyleSheet, View } from 'react-native';
import { previewBox, previewDrawing } from '@/src/state/gestures';
import { pluginTouch } from '@/src/state/plugin-touch';

/**
 * Hands one finger on the preview to the plugins that listen to `events.touches`. It covers the preview and takes one-finger touches, so
 * mount it only while a plugin listens. A second finger goes to the preview, so the photo can still be pinched and panned.
 */
export function PluginTouchOverlay() {
  useEffect(() => {
    previewDrawing.next(true);
    return () => previewDrawing.next(false);
  }, []);

  const responder = useMemo(() => {
    // Where this layer is on the screen, taken from the first touch, which lands on the layer itself.
    const origin = { x: 0, y: 0 };
    const at = (event: { nativeEvent: { pageX: number; pageY: number } }) => ({
      x: event.nativeEvent.pageX - origin.x,
      y: event.nativeEvent.pageY - origin.y,
    });
    return PanResponder.create({
      onStartShouldSetPanResponder: () => true,
      onMoveShouldSetPanResponder: () => true,
      // A second finger belongs to the preview's pinch responder.
      onPanResponderTerminationRequest: () => true,
      onPanResponderGrant: event => {
        origin.x = event.nativeEvent.pageX - event.nativeEvent.locationX;
        origin.y = event.nativeEvent.pageY - event.nativeEvent.locationY;
        pluginTouch.next({ type: 'down', ...at(event) });
      },
      onPanResponderMove: event => {
        if (event.nativeEvent.touches.length > 1) return;
        pluginTouch.next({ type: 'move', ...at(event) });
      },
      onPanResponderRelease: event => pluginTouch.next({ type: 'up', ...at(event) }),
      onPanResponderTerminate: event => pluginTouch.next({ type: 'cancel', ...at(event) }),
    });
  }, []);

  return <View style={StyleSheet.absoluteFill} {...responder.panHandlers} />;
}
