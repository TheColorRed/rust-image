import { clamp, hexToRgb, hsbToRgb, readableOn, rgbToHex, rgbToHsb, type Hsb, type Rgb } from '@/src/lib/color/color-math';
import { DEFAULT_SWATCHES } from '@/src/lib/color/color-swatches';
import { COLOR_SURFACE_RING, COLOR_SURFACE_SHADOW } from '@/src/lib/color/surface-colors';
import { useOwning } from '@/src/lib/native/native-handles';
import { useTheme } from '@/src/lib/ui/theme';
import type { ColorPickerRequest, ColorPickerResult } from '@/src/state/color-picker';
import { ColorPicker } from '@alakazam/mobile';
import RNSlider from '@react-native-community/slider';
import { VesselView } from '@vessel/react-native';
import { useEffect, useMemo, useRef, useState } from 'react';
import { Modal, PixelRatio, Pressable, ScrollView, StyleSheet, Text, useWindowDimensions, View } from 'react-native';
import { KeyboardScrollView, RevealInput } from '@/src/components/keyboard-scroll';

/** The width of the bar that picks which color it is, and the room between it and the square. */
const HUE_BAR_WIDTH = 56;
const GAP = 12;
const CARD_PADDING = 14;
/** The height of the bar that picks how solid the color is, and the room above it. These are the native surface's own measures (`ColorPicker`). */
const STRENGTH_HEIGHT = 28;

const same = (a: Rgb, b: Rgb) => a[0] === b[0] && a[1] === b[1] && a[2] === b[2];
const css = (rgb: Rgb) => `rgb(${rgb[0]}, ${rgb[1]}, ${rgb[2]})`;

/**
 * A box for one number, such as red or colorfulness. What is typed is taken when the person is done with the box, and a number that is not allowed is put back.
 * With `slider` the box has a slider to its left, so the number can be dragged to without typing; the box takes the whole line then, and every move of the
 * slider is a new number for `onCommit`.
 */
export function NumberBox({
  label,
  value,
  max,
  suffix,
  slider = false,
  onCommit,
}: {
  label: string;
  value: number;
  max: number;
  suffix?: string;
  slider?: boolean;
  onCommit: (value: number) => void;
}) {
  const { colors } = useTheme();
  const [text, setText] = useState(String(value));
  // The color can change from somewhere else (the square, a swatch), and then the box shows it.
  useEffect(() => setText(String(value)), [value]);
  const commit = () => {
    const typed = Number.parseFloat(text);
    if (Number.isFinite(typed)) onCommit(clamp(Math.round(typed), 0, max));
    else setText(String(value));
  };
  return (
    <View style={[styles.field, slider ? styles.fieldWithSlider : null]}>
      <Text numberOfLines={1} style={[styles.fieldLabel, slider ? styles.fieldLabelWithSlider : null, { color: colors.textSecondary }]}>
        {label}
      </Text>
      {slider ? (
        <RNSlider
          accessibilityLabel={`${label} slider`}
          style={styles.fieldSlider}
          minimumValue={0}
          maximumValue={max}
          step={1}
          value={value}
          minimumTrackTintColor={colors.accent}
          maximumTrackTintColor={colors.chip}
          thumbTintColor={colors.accent}
          onValueChange={next => {
            const rounded = Math.round(next);
            if (rounded !== value) onCommit(rounded);
          }}
        />
      ) : null}
      <RevealInput
        accessibilityLabel={label}
        value={text}
        onChangeText={setText}
        onEndEditing={commit}
        onSubmitEditing={commit}
        keyboardType="number-pad"
        selectTextOnFocus
        maxLength={3}
        style={[styles.input, { color: colors.textPrimary, backgroundColor: colors.chip, borderColor: colors.border }]}
      />
      {suffix ? <Text style={[styles.suffix, { color: colors.textSecondary }]}>{suffix}</Text> : null}
    </View>
  );
}

/**
 * The color picker: a square to drag a finger over for how colorful and how bright, a bar for which color, the color it makes beside the one it
 * started as, named colors for the common choices, and boxes to type a color exactly. It is shown by `ColorPickerHost` when something asks with
 * `pickColor`, and answers with the color on OK or nothing on Cancel.
 */
export function ColorPickerDialog({ request, onDone }: { request: ColorPickerRequest; onDone: (result: ColorPickerResult | null) => void }) {
  const { colors } = useTheme();
  const { width: windowWidth, height: windowHeight } = useWindowDimensions();
  const { options } = request;
  const original = useMemo<Rgb>(() => options.value ?? [255, 255, 255], [options.value]);
  const originalAlpha = options.alpha ?? 255;
  const swatches = options.swatches ?? DEFAULT_SWATCHES;

  const [hsb, setHsb] = useState<Hsb>(() => rgbToHsb(original));
  const [alpha, setAlpha] = useState(originalAlpha);
  const rgb = useMemo(() => hsbToRgb(hsb), [hsb]);
  const [hex, setHex] = useState(rgbToHex(rgb));
  useEffect(() => setHex(rgbToHex(rgb)), [rgb]);

  // A caller that shows the color at once is told as it changes, but not for the color the picker opened on.
  const opened = useRef(true);
  useEffect(() => {
    if (opened.current) {
      opened.current = false;
      return;
    }
    options.onChange?.({ rgb, alpha });
    // The callback is the caller's and does not change what the picker shows.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [rgb, alpha]);

  /** Sets the color from red, green and blue, keeping the hue when the color is a grey or black, which has none. */
  const setRgb = (next: Rgb) => setHsb(previous => rgbToHsb(next, previous));

  const cardWidth = Math.min(windowWidth - 24, 440);
  // The square takes the room the bar leaves, and no more than the screen has height for, so the whole picker still fits.
  const squareSize = Math.max(140, cardWidth - CARD_PADDING * 2 - HUE_BAR_WIDTH - GAP);

  // The square, the bar of colors and the bar of strengths are one native surface (`ColorPicker`: abra draws them and Vessel renders them and takes the
  // touches). It holds the color while a finger is on it, which is read on each move, and is told the color when it changes from anywhere else.
  const hasStrength = options.alpha !== undefined;
  const surface = useOwning(
    keep =>
      keep(
        new ColorPicker(
          hsb.hue,
          hsb.saturation,
          hsb.brightness,
          hasStrength,
          alpha,
          new Uint8Array(COLOR_SURFACE_RING).buffer,
          new Uint8Array(COLOR_SURFACE_SHADOW).buffer,
          PixelRatio.get(),
        ),
      ),
    // The surface starts as the picker opened and is changed after that with `setColor` and `setAlpha`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [],
  );
  useEffect(() => {
    surface?.setColor(hsb.hue, hsb.saturation, hsb.brightness);
    surface?.setAlpha(alpha);
  }, [surface, hsb, alpha]);
  const readSurface = () => {
    if (!surface) return;
    setHsb({ hue: surface.hue(), saturation: surface.saturation(), brightness: surface.brightness() });
    setAlpha(surface.alpha());
  };

  const commitHex = () => {
    const typed = hexToRgb(hex);
    if (typed) setRgb(typed);
    else setHex(rgbToHex(rgb));
  };

  return (
    <Modal transparent animationType="fade" statusBarTranslucent visible onRequestClose={() => onDone(null)}>
      <View style={styles.backdrop}>
        <View style={[styles.card, { width: cardWidth, backgroundColor: colors.surface, borderColor: colors.border, maxHeight: windowHeight - 40 }]}>
          <Text style={[styles.title, { color: colors.textPrimary }]}>{options.title ?? 'Pick a color'}</Text>
          <KeyboardScrollView showsVerticalScrollIndicator={false}>
            <ScrollView horizontal showsHorizontalScrollIndicator={false} contentContainerStyle={styles.swatchRow}>
              {swatches.map(swatch => {
                const chosen = same(swatch.rgb, rgb);
                return (
                  <Pressable
                    key={swatch.label}
                    accessibilityRole="button"
                    accessibilityLabel={swatch.label}
                    accessibilityState={{ selected: chosen }}
                    onPress={() => setRgb(swatch.rgb)}
                    style={styles.swatch}
                  >
                    <View
                      style={[
                        styles.swatchColor,
                        { backgroundColor: css(swatch.rgb), borderColor: chosen ? colors.accent : colors.border, borderWidth: chosen ? 3 : 1 },
                      ]}
                    />
                    <Text numberOfLines={1} style={[styles.swatchLabel, { color: chosen ? colors.textPrimary : colors.textSecondary }]}>
                      {swatch.label}
                    </Text>
                  </Pressable>
                );
              })}
            </ScrollView>

            <View
              style={{ width: cardWidth - CARD_PADDING * 2, height: squareSize + (hasStrength ? STRENGTH_HEIGHT + GAP : 0) }}
              onTouchMove={readSurface}
              onTouchEnd={readSurface}
              onTouchCancel={readSurface}
            >
              {VesselView && surface ? <VesselView source={surface} style={StyleSheet.absoluteFill} /> : null}
            </View>

            <View style={styles.detailRow}>
              <View style={styles.preview}>
                <Text style={[styles.previewLabel, { color: colors.textSecondary }]}>New</Text>
                <View style={[styles.previewColor, { backgroundColor: css(rgb), opacity: alpha / 255, borderColor: colors.border }]} />
                <Pressable
                  accessibilityRole="button"
                  accessibilityLabel="Go back to the first color"
                  onPress={() => {
                    setRgb(original);
                    setAlpha(originalAlpha);
                  }}
                  style={[styles.previewColor, { backgroundColor: css(original), opacity: originalAlpha / 255, borderColor: colors.border }]}
                />
                <Text style={[styles.previewLabel, { color: colors.textSecondary }]}>Current</Text>
              </View>
              <View style={styles.fields}>
                <View style={styles.fieldColumn}>
                  <NumberBox label="Color" value={Math.round(hsb.hue % 360)} max={360} suffix="°" onCommit={hue => setHsb(previous => ({ ...previous, hue }))} />
                  <NumberBox
                    label="Colorfulness"
                    value={Math.round(hsb.saturation * 100)}
                    max={100}
                    suffix="%"
                    onCommit={value => setHsb(previous => ({ ...previous, saturation: value / 100 }))}
                  />
                  <NumberBox
                    label="Brightness"
                    value={Math.round(hsb.brightness * 100)}
                    max={100}
                    suffix="%"
                    onCommit={value => setHsb(previous => ({ ...previous, brightness: value / 100 }))}
                  />
                </View>
                <View style={styles.fieldColumn}>
                  <NumberBox label="Red" value={rgb[0]} max={255} onCommit={value => setRgb([value, rgb[1], rgb[2]])} />
                  <NumberBox label="Green" value={rgb[1]} max={255} onCommit={value => setRgb([rgb[0], value, rgb[2]])} />
                  <NumberBox label="Blue" value={rgb[2]} max={255} onCommit={value => setRgb([rgb[0], rgb[1], value])} />
                </View>
              </View>
            </View>

            <View style={styles.hexRow}>
              <Text style={[styles.fieldLabel, { color: colors.textSecondary }]}>Web color</Text>
              <RevealInput
                accessibilityLabel="Web color"
                value={hex}
                onChangeText={setHex}
                onEndEditing={commitHex}
                onSubmitEditing={commitHex}
                autoCapitalize="none"
                autoCorrect={false}
                selectTextOnFocus
                maxLength={7}
                style={[styles.hexInput, { color: colors.textPrimary, backgroundColor: colors.chip, borderColor: colors.border }]}
              />
              <View style={[styles.hexSwatch, { backgroundColor: css(rgb), borderColor: colors.border }]}>
                <Text style={{ color: readableOn(rgb), fontSize: 11 }}>{rgbToHex(rgb)}</Text>
              </View>
            </View>
          </KeyboardScrollView>

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
              onPress={() => onDone({ rgb, alpha: options.alpha === undefined ? 255 : alpha })}
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
  swatchRow: { gap: 10, paddingBottom: 10 },
  swatch: { alignItems: 'center', width: 52 },
  swatchColor: { width: 34, height: 34, borderRadius: 17 },
  swatchLabel: { fontSize: 10, marginTop: 3 },
  detailRow: { flexDirection: 'row', gap: 14, marginTop: 14 },
  preview: { width: 64, alignItems: 'stretch' },
  previewLabel: { fontSize: 11, textAlign: 'center' },
  previewColor: { height: 34, borderWidth: StyleSheet.hairlineWidth },
  fields: { flex: 1, flexDirection: 'row', gap: 14 },
  fieldColumn: { flex: 1, gap: 6 },
  field: { flexDirection: 'row', alignItems: 'center', gap: 4 },
  fieldLabel: { fontSize: 12, flexShrink: 1, minWidth: 0, flexGrow: 1 },
  fieldWithSlider: { width: '100%' },
  fieldLabelWithSlider: { flexGrow: 0, width: 64 },
  fieldSlider: { flex: 1, height: 30 },
  input: { width: 46, height: 30, borderWidth: StyleSheet.hairlineWidth, borderRadius: 6, textAlign: 'center', paddingVertical: 0, fontSize: 14 },
  suffix: { width: 12, fontSize: 12 },
  hexRow: { flexDirection: 'row', alignItems: 'center', gap: 8, marginTop: 12 },
  hexInput: { width: 96, height: 32, borderWidth: StyleSheet.hairlineWidth, borderRadius: 6, textAlign: 'center', paddingVertical: 0, fontSize: 14 },
  hexSwatch: { flex: 1, height: 32, borderRadius: 6, borderWidth: StyleSheet.hairlineWidth, alignItems: 'center', justifyContent: 'center' },
  buttons: { flexDirection: 'row', gap: 10, justifyContent: 'flex-end', marginTop: 4 },
  button: { minWidth: 92, paddingVertical: 10, paddingHorizontal: 16, borderRadius: 10, borderWidth: StyleSheet.hairlineWidth, alignItems: 'center' },
  buttonText: { fontSize: 15, fontWeight: '600' },
});
