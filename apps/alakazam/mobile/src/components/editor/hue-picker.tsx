import { HueColorPicker } from '@alakazam/mobile';
import { VesselView } from '@vessel/react-native';
import { useEffect, useMemo } from 'react';
import { StyleSheet, View } from 'react-native';

/**
 * The bar of hues for a control that picks a color. Vessel draws it and handles the dragging; this only tells the screen
 * where the handle ended, to apply the edit.
 */
export function HuePicker({ value, onChange, onRelease }: {
  /** Where the handle starts, 0 to 1 along the bar. */
  value: number;
  /** The handle moved while the finger is down: it is where it is now. */
  onChange: (value: number) => void;
  /** The touch ended with the handle at this value. */
  onRelease: (value: number) => void;
}) {
  const picker = useMemo(() => new HueColorPicker(value), []);
  useEffect(() => () => picker.uniffiDestroy(), [picker]);

  if (!VesselView) return null;
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
