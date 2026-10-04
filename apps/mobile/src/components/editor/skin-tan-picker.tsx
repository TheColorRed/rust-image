import { SkinTanColorPicker, type ImagePreview } from '@alakazam/mobile';
import { VesselView } from '@vessel/react-native';
import { useEffect, useMemo } from 'react';
import { StyleSheet, View } from 'react-native';

/**
 * The bar of tans for the Skin Tan control. Vessel draws it and handles the dragging; each position goes straight to the
 * live `preview`, and this only tells the screen when a drag starts (to show the preview) and where it ended (to apply the
 * edit).
 */
export function SkinTanPicker({
  preview,
  value,
  onTouch,
  onRelease,
}: {
  /** The live preview the tans are shown on. */
  preview: ImagePreview | null;
  /** Where the handle starts, 0 to 1. */
  value: number;
  /** A touch began. */
  onTouch: () => void;
  /** The touch ended with the handle at this value. */
  onRelease: (value: number) => void;
}) {
  // Made again when the preview changes, so it always talks to the preview that is open.
  const picker = useMemo(() => (preview ? new SkinTanColorPicker(preview, value) : null), [preview]);
  useEffect(() => () => picker?.uniffiDestroy(), [picker]);

  if (!VesselView) return null;
  const release = () => picker && onRelease(picker.value());
  return (
    <View style={styles.bar} onTouchStart={onTouch} onTouchEnd={release} onTouchCancel={release}>
      <VesselView source={picker} style={StyleSheet.absoluteFill} />
    </View>
  );
}

const styles = StyleSheet.create({
  bar: { flex: 1, height: 36 },
});
