import { NumberBox } from '@/src/components/color-picker/color-picker-dialog';
import { rgbToHex, type Rgb } from '@/src/lib/color/color-math';
import { sortedValue } from '@/src/lib/color/gradient-math';
import { GRADIENT_SURFACE_ACCENT, GRADIENT_SURFACE_OUTLINE } from '@/src/lib/color/surface-colors';
import { useOwning } from '@/src/lib/native/native-handles';
import { useTheme } from '@/src/lib/ui/theme';
import { pickColor } from '@/src/state/color-picker';
import type { GradientPickerRequest, GradientValue } from '@/src/state/gradient-picker';
import { GradientEditor } from '@alakazam/mobile';
import { VesselView } from '@vessel/react-native';
import { useState } from 'react';
import { Modal, PixelRatio, Pressable, StyleSheet, Text, useWindowDimensions, View } from 'react-native';

const CARD_PADDING = 14;
/** The height of the surface: a strip of strength stops, the bar and a strip of color stops. */
const SURFACE_HEIGHT = 104;

type Selected = { kind: 'color' | 'alpha'; index: number };

/** The color stops as the `place, red, green, blue, ...` list the native editor takes. */
const flatColors = (value: GradientValue) => value.colors.flatMap(stop => [stop.at, ...stop.color]);

/** The strength stops as the `place, alpha, ...` list the native editor takes. */
const flatAlphas = (value: GradientValue) => value.alphas.flatMap(stop => [stop.at, stop.alpha]);

/** The lists of the native editor as a gradient. */
const gradientOf = (colors: number[], alphas: number[]): GradientValue => ({
  colors: Array.from({ length: colors.length / 4 }, (_, at) => ({ at: colors[at * 4], color: [colors[at * 4 + 1], colors[at * 4 + 2], colors[at * 4 + 3]] as Rgb })),
  alphas: Array.from({ length: alphas.length / 2 }, (_, at) => ({ at: alphas[at * 2], alpha: alphas[at * 2 + 1] })),
});

/**
 * The gradient editor, which opens from a gradient control the way the color picker opens from a color. The bar over its checkerboard, the strips of
 * stops above and below it, and the touches on them are a native surface (`GradientEditor`: abra draws the picture and Vessel renders it); this is the
 * rest: the settings of the selected stop (its color, strength and location), Remove, OK and Cancel. It is shown by `GradientPickerHost` when something
 * asks with `pickGradient`, and answers with the gradient on OK or nothing on Cancel.
 */
export function GradientPickerDialog({ request, onDone }: { request: GradientPickerRequest; onDone: (result: GradientValue | null) => void }) {
  const { colors } = useTheme();
  const { width: windowWidth, height: windowHeight } = useWindowDimensions();
  const { options } = request;

  const editor = useOwning(keep => {
    const start = sortedValue(options.value);
    return keep(
      new GradientEditor(
        flatColors(start),
        flatAlphas(start),
        new Uint8Array(GRADIENT_SURFACE_ACCENT).buffer,
        new Uint8Array(GRADIENT_SURFACE_OUTLINE).buffer,
        PixelRatio.get(),
      ),
    );
  }, []);
  const [value, setValue] = useState<GradientValue>(() => JSON.parse(JSON.stringify(sortedValue(options.value))));
  const [selected, setSelected] = useState<Selected | null>({ kind: 'color', index: 0 });

  /** Takes what the surface holds, which changes while a finger is on it, and tells a caller that shows the gradient at once. */
  const read = () => {
    if (!editor) return;
    const next = gradientOf(editor.colors(), editor.alphas());
    setValue(next);
    const kind = editor.selectedKind();
    setSelected(kind < 0 ? null : { kind: kind === 0 ? 'color' : 'alpha', index: editor.selectedIndex() });
    options.onChange?.(sortedValue(next));
  };

  /** Changes the gradient from here (a field, Remove) and has the surface show it. */
  const change = (next: GradientValue, nextSelected: Selected | null = selected) => {
    setValue(next);
    setSelected(nextSelected);
    editor?.setColors(flatColors(next));
    editor?.setAlphas(flatAlphas(next));
    editor?.setSelected(nextSelected ? (nextSelected.kind === 'color' ? 0 : 1) : -1, nextSelected?.index ?? -1);
    options.onChange?.(sortedValue(next));
  };

  const cardWidth = Math.min(windowWidth - 24, 440);
  const barWidth = cardWidth - CARD_PADDING * 2;

  const stop = selected ? (selected.kind === 'color' ? value.colors[selected.index] : value.alphas[selected.index]) : undefined;
  const count = selected ? (selected.kind === 'color' ? value.colors.length : value.alphas.length) : 0;
  const setSelectedStop = (changes: Partial<{ at: number; color: Rgb; alpha: number }>) => {
    if (!selected) return;
    change(
      selected.kind === 'color'
        ? { ...value, colors: value.colors.map((each, index) => (index === selected.index ? { ...each, ...changes } : each)) }
        : { ...value, alphas: value.alphas.map((each, index) => (index === selected.index ? { ...each, ...changes } : each)) },
    );
  };
  const remove = () => {
    if (!selected || count <= 1) return;
    change(
      selected.kind === 'color'
        ? { ...value, colors: value.colors.filter((_, index) => index !== selected.index) }
        : { ...value, alphas: value.alphas.filter((_, index) => index !== selected.index) },
      { kind: selected.kind, index: 0 },
    );
  };
  const chooseColor = async () => {
    if (selected?.kind !== 'color' || !stop) return;
    const picked = await pickColor({ title: 'Stop color', value: (stop as GradientValue['colors'][number]).color, swatches: options.swatches });
    if (picked) setSelectedStop({ color: picked.rgb });
  };

  return (
    <Modal transparent animationType="fade" statusBarTranslucent visible onRequestClose={() => onDone(null)}>
      <View style={styles.backdrop}>
        <View style={[styles.card, { width: cardWidth, backgroundColor: colors.surface, borderColor: colors.border, maxHeight: windowHeight - 40 }]}>
          <Text style={[styles.title, { color: colors.textPrimary }]}>{options.title ?? 'Edit the gradient'}</Text>

          <View style={{ width: barWidth, height: SURFACE_HEIGHT }} onTouchEnd={read} onTouchCancel={read}>
            {VesselView && editor ? <VesselView source={editor} style={StyleSheet.absoluteFill} /> : null}
          </View>

          <Text style={[styles.hint, { color: colors.textSecondary }]}>
            Strength stops are above the bar and color stops below it. Drag a stop to move it, or touch an empty place to add one.
          </Text>

          {selected && stop ? (
            <View style={styles.stopRow}>
              <Text style={[styles.stopTitle, { color: colors.textPrimary }]}>{selected.kind === 'color' ? 'Color stop' : 'Strength stop'}</Text>
              {selected.kind === 'color' ? (
                <Pressable
                  accessibilityRole="button"
                  accessibilityLabel="Stop color"
                  onPress={chooseColor}
                  style={[styles.colorButton, { backgroundColor: colors.chip, borderColor: colors.border }]}
                >
                  <View style={[styles.colorSwatch, { backgroundColor: `rgb(${(stop as GradientValue['colors'][number]).color.join(', ')})`, borderColor: colors.border }]} />
                  <Text style={{ color: colors.textPrimary, fontSize: 14 }}>{rgbToHex((stop as GradientValue['colors'][number]).color)}</Text>
                </Pressable>
              ) : (
                <NumberBox
                  label="Strength"
                  value={Math.round(((stop as GradientValue['alphas'][number]).alpha / 255) * 100)}
                  max={100}
                  suffix="%"
                  slider
                  onCommit={percent => setSelectedStop({ alpha: Math.round((percent / 100) * 255) })}
                />
              )}
              <NumberBox label="Location" value={Math.round(stop.at * 100)} max={100} suffix="%" slider onCommit={percent => setSelectedStop({ at: percent / 100 })} />
              <Pressable
                accessibilityRole="button"
                accessibilityLabel="Remove stop"
                accessibilityState={{ disabled: count <= 1 }}
                disabled={count <= 1}
                onPress={remove}
                style={[styles.removeButton, { backgroundColor: colors.chip, borderColor: colors.border, opacity: count <= 1 ? 0.4 : 1 }]}
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
              onPress={() => onDone(sortedValue(value))}
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
  hint: { fontSize: 12 },
  stopRow: { flexDirection: 'row', alignItems: 'center', flexWrap: 'wrap', gap: 10 },
  stopTitle: { fontSize: 13, fontWeight: '600' },
  colorButton: { flexDirection: 'row', alignItems: 'center', gap: 8, borderWidth: 1, borderRadius: 8, paddingHorizontal: 8, paddingVertical: 5 },
  colorSwatch: { width: 22, height: 22, borderRadius: 5, borderWidth: 1 },
  removeButton: { borderWidth: StyleSheet.hairlineWidth, borderRadius: 8, paddingHorizontal: 10, paddingVertical: 7 },
  buttons: { flexDirection: 'row', gap: 10, justifyContent: 'flex-end', marginTop: 4 },
  button: { minWidth: 92, paddingVertical: 10, paddingHorizontal: 16, borderRadius: 10, borderWidth: StyleSheet.hairlineWidth, alignItems: 'center' },
  buttonText: { fontSize: 15, fontWeight: '600' },
});
