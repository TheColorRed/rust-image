import { NumberBox } from '@/src/components/color-picker/color-picker-dialog';
import { clamp } from '@/src/lib/color/color-math';
import { CURVE_SURFACE_COLORS } from '@/src/lib/color/surface-colors';
import { sortedCurve } from '@/src/lib/curve/curve-samples';
import { useOwning } from '@/src/lib/native/native-handles';
import { useTheme } from '@/src/lib/ui/theme';
import type { CurvePickerRequest, CurveValue } from '@/src/state/curve-picker';
import { CurveEditor } from '@alakazam/mobile';
import { VesselView } from '@vessel/react-native';
import { useState } from 'react';
import { Modal, PixelRatio, Pressable, ScrollView, StyleSheet, Text, useWindowDimensions, View } from 'react-native';

const CARD_PADDING = 14;
const PLOT_HEIGHT = 200;

/** The points as the `x, y, x, y, ...` list the native editor takes. */
const flat = (points: CurveValue) => points.flatMap(([x, y]) => [x, y]);

/** The `x, y, x, y, ...` list of the native editor as points. */
const pairs = (list: number[]): CurveValue => Array.from({ length: list.length / 2 }, (_, at) => [list[at * 2], list[at * 2 + 1]]);

/**
 * The curve editor, which opens from a curve control the way the color picker opens from a color. The curve and its points are drawn, and the touches on
 * them taken, by a native surface (`CurveEditor`: abra draws the picture and Vessel renders it); this is the rest: the shapes to start from, the fields of
 * the selected point, and Remove, OK and Cancel. It is shown by `CurvePickerHost` when something asks with `pickCurve`, and answers with the curve on OK or
 * nothing on Cancel.
 */
export function CurvePickerDialog({ request, onDone }: { request: CurvePickerRequest; onDone: (result: CurveValue | null) => void }) {
  const { colors } = useTheme();
  const { width: windowWidth, height: windowHeight } = useWindowDimensions();
  const { options } = request;
  const { min, max } = options;
  const span = max > min ? max - min : 1;

  const editor = useOwning(keep => keep(new CurveEditor(flat(options.value), min, max, CURVE_SURFACE_COLORS.map(color => new Uint8Array(color).buffer), PixelRatio.get())), []);
  const [points, setPoints] = useState<CurveValue>(() => options.value.map(point => [...point] as [number, number]));
  const [selected, setSelected] = useState(0);

  /** Takes what the surface holds, which changes while a finger is on it, and tells a caller that shows the curve at once. */
  const read = () => {
    if (!editor) return;
    const next = pairs(editor.points());
    setPoints(next);
    setSelected(editor.selected());
    options.onChange?.(sortedCurve(next));
  };

  /** Changes the points from here (a field, a shape, Remove) and has the surface show them. */
  const change = (next: CurveValue, nextSelected = selected) => {
    setPoints(next);
    setSelected(nextSelected);
    editor?.setPoints(flat(next));
    editor?.setSelected(nextSelected);
    options.onChange?.(sortedCurve(next));
  };

  const cardWidth = Math.min(windowWidth - 24, 440);
  const plotWidth = cardWidth - CARD_PADDING * 2;
  const point = points[selected];
  const setSelectedPoint = (changes: { x?: number; y?: number }) =>
    change(points.map((each, index) => (index === selected ? ([clamp(changes.x ?? each[0], 0, 1), clamp(changes.y ?? each[1], min, max)] as [number, number]) : each)));

  return (
    <Modal transparent animationType="fade" statusBarTranslucent visible onRequestClose={() => onDone(null)}>
      <View style={styles.backdrop}>
        <View style={[styles.card, { width: cardWidth, backgroundColor: colors.surface, borderColor: colors.border, maxHeight: windowHeight - 40 }]}>
          <Text style={[styles.title, { color: colors.textPrimary }]}>{options.title ?? 'Edit the curve'}</Text>

          {options.presets?.length ? (
            <ScrollView horizontal showsHorizontalScrollIndicator={false} contentContainerStyle={styles.presets}>
              {options.presets.map(preset => (
                <Pressable
                  key={preset.label}
                  accessibilityRole="button"
                  accessibilityLabel={preset.label}
                  onPress={() => change(preset.value.map(each => [each[0], clamp(each[1], min, max)] as [number, number]), 0)}
                  style={[styles.preset, { backgroundColor: colors.chip, borderColor: colors.border }]}
                >
                  <Text style={{ color: colors.textPrimary, fontSize: 13 }}>{preset.label}</Text>
                </Pressable>
              ))}
            </ScrollView>
          ) : null}

          <View style={{ width: plotWidth, height: PLOT_HEIGHT }} onTouchMove={read} onTouchEnd={read} onTouchCancel={read}>
            {VesselView && editor ? <VesselView source={editor} style={StyleSheet.absoluteFill} /> : null}
          </View>

          <Text style={[styles.hint, { color: colors.textSecondary }]}>Drag a point to move it, or touch an empty place to add one.</Text>

          {point ? (
            <View style={styles.pointRow}>
              <NumberBox label="Time" value={Math.round(point[0] * 100)} max={100} suffix="%" onCommit={percent => setSelectedPoint({ x: percent / 100 })} />
              <NumberBox
                label="Value"
                value={Math.round(((point[1] - min) / span) * 100)}
                max={100}
                suffix="%"
                onCommit={percent => setSelectedPoint({ y: min + (percent / 100) * span })}
              />
              <Pressable
                accessibilityRole="button"
                accessibilityLabel="Remove point"
                accessibilityState={{ disabled: points.length <= 1 }}
                disabled={points.length <= 1}
                onPress={() => change(points.filter((_, index) => index !== selected), 0)}
                style={[styles.removeButton, { backgroundColor: colors.chip, borderColor: colors.border, opacity: points.length <= 1 ? 0.4 : 1 }]}
              >
                <Text style={{ color: colors.textPrimary, fontSize: 13 }}>Remove</Text>
              </Pressable>
            </View>
          ) : null}

          <View style={styles.buttons}>
            <Pressable
              accessibilityRole="button"
              onPress={() => onDone(null)}
              style={[styles.button, { backgroundColor: colors.chip, borderColor: colors.border }]}
            >
              <Text style={[styles.buttonText, { color: colors.textPrimary }]}>Cancel</Text>
            </Pressable>
            <Pressable
              accessibilityRole="button"
              onPress={() => onDone(sortedCurve(points))}
              style={[styles.button, { backgroundColor: colors.accent, borderColor: colors.accent }]}
            >
              <Text style={[styles.buttonText, { color: '#ffffff' }]}>OK</Text>
            </Pressable>
          </View>
        </View>
      </View>
    </Modal>
  );
}

const styles = StyleSheet.create({
  backdrop: { flex: 1, alignItems: 'center', justifyContent: 'center', backgroundColor: 'rgba(0,0,0,0.55)' },
  card: { borderRadius: 14, borderWidth: StyleSheet.hairlineWidth, padding: CARD_PADDING, gap: 10 },
  title: { fontSize: 16, fontWeight: '600' },
  presets: { gap: 8 },
  preset: { borderWidth: StyleSheet.hairlineWidth, borderRadius: 8, paddingHorizontal: 10, paddingVertical: 6 },
  hint: { fontSize: 12 },
  pointRow: { flexDirection: 'row', alignItems: 'center', flexWrap: 'wrap', gap: 10 },
  removeButton: { borderWidth: StyleSheet.hairlineWidth, borderRadius: 8, paddingHorizontal: 10, paddingVertical: 7 },
  buttons: { flexDirection: 'row', gap: 10, justifyContent: 'flex-end', marginTop: 4 },
  button: { minWidth: 92, paddingVertical: 10, paddingHorizontal: 16, borderRadius: 10, borderWidth: StyleSheet.hairlineWidth, alignItems: 'center' },
  buttonText: { fontSize: 15, fontWeight: '600' },
});
