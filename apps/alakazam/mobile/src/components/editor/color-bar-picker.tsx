// @refresh reset
import { ColorBarPicker as ColorBar } from '@alakazam/mobile';
import { VesselView } from '@vessel/react-native';
import { StyleSheet, View } from 'react-native';
import { useOwning } from '@/src/lib/native/native-handles';

/**
 * A bar of colors for a control that picks one: a hue, a shade, a tan. Vessel draws it and handles the dragging; this only tells
 * the screen where the handle is (`onChange` while it moves, `onRelease` when the finger lifts), to preview and apply the edit.
 */
export function ColorBarPicker({
  colors,
  value,
  onChange,
  onRelease,
}: {
  /** The colors along the bar, each as red, green and blue from 0 to 255, evenly spaced. */
  colors: number[][];
  /** Where the handle starts, 0 to 1 along the bar. */
  value: number;
  /** The handle moved while the finger is down: it is where it is now. */
  onChange: (value: number) => void;
  /** The touch ended with the handle at this value. */
  onRelease: (value: number) => void;
}) {
  const picker = useOwning(
    keep =>
      keep(
        new ColorBar(
          value,
          colors.map(color => new Uint8Array(color).buffer),
        ),
      ),
    [],
  );

  if (!VesselView || !picker) return null;
  const release = () => onRelease(picker.value());
  return (
    <View style={styles.bar} onTouchMove={() => onChange(picker.value())} onTouchEnd={release} onTouchCancel={release}>
      <VesselView source={picker} style={StyleSheet.absoluteFill} />
    </View>
  );
}

const styles = StyleSheet.create({
  bar: { flex: 1, height: 36 },
});
