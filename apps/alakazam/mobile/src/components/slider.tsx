import RNSlider from '@react-native-community/slider';
import { RotateCcw } from 'lucide-react-native';
import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { Pressable, StyleSheet, Text, View } from 'react-native';
import { useTheme, type ThemeColors } from '@/src/lib/theme';

const DEBOUNCE_MS = 300;
/**
 * Space kept on each side of the track, beyond the screen padding, so a drag that starts near the thumb's end stops
 * short of the edge where the system's back swipe begins.
 */
const TRACK_EDGE_MARGIN = 20;

const formatValue = (value: number, step: number) => {
  const rounded = step >= 1 ? Math.round(value) : Math.round(value * 10) / 10;
  return rounded > 0 ? `+${rounded}` : `${rounded}`;
};

export interface SliderProps {
  /** The minimum value of the slider. */
  min: number;
  /** The maximum value of the slider. */
  max: number;
  /** The step between selectable values. Use a fractional step (e.g. 0.1) for finer controls. */
  step?: number;
  /** The label for the slider. */
  label: string;
  /** Whether the slider should show a reset button. */
  reset: boolean;
  /** The value the slider starts at. */
  initialValue?: number;
  /** What the reset button returns the slider to. Defaults to `initialValue`, which is wrong when the slider starts from an already-applied value. */
  resetValue?: number;
  /** The trigger type for the slider. Debounce triggers after a delay, release triggers on release. */
  triggerType?: 'live' | 'debounce' | 'release';
  /** Trigged when the slider value changes (ignores `triggerType`). */
  onChange?: (value: number) => void;
  /** Trigged when the slider is reset. */
  onReset?: () => void;
  /** Trigged when the slider value is triggered based on `triggerType`. */
  onTrigger?: (value: number) => void;
  /** Shown at the start of the header line, before the label. */
  leading?: ReactNode;
  /** Shown at the end of the header line, after the reset button. */
  trailing?: ReactNode;
}

/**
 * A labelled slider with a live value readout and an optional reset-to-original button. The label, value and reset
 * button sit on a header line, and the track gets its own line, almost the full width.
 */
export function Slider({
  min,
  max,
  step = 1,
  label,
  reset,
  initialValue = 0,
  resetValue = initialValue,
  triggerType = 'live',
  onChange,
  onReset,
  onTrigger,
  leading,
  trailing,
}: SliderProps) {
  const { colors } = useTheme();
  const styles = useMemo(() => createStyles(colors), [colors]);
  const [value, setValue] = useState(initialValue);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(
    () => () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    },
    [],
  );

  function handleChange(next: number) {
    setValue(next);
    onChange?.(next);
    if (triggerType === 'debounce') {
      if (debounceRef.current) clearTimeout(debounceRef.current);
      debounceRef.current = setTimeout(() => onTrigger?.(next), DEBOUNCE_MS);
    }
    if (triggerType === 'live') {
      onTrigger?.(next);
    }
  }

  function handleSlidingComplete(next: number) {
    if (triggerType !== 'release') return;
    if (debounceRef.current) clearTimeout(debounceRef.current);
    onTrigger?.(next);
  }

  function handleReset() {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    setValue(resetValue);
    // The live preview keeps showing the last dragged value until told otherwise, and it covers the photo, so the
    // committed reset underneath would never be seen.
    onChange?.(resetValue);
    onReset?.();
  }

  return (
    <View>
      <View style={styles.header}>
        {leading}
        <Text style={styles.label} numberOfLines={1}>
          {label}
        </Text>
        <Text style={styles.value}>{formatValue(value, step)}</Text>
        {reset && (
          <Pressable onPress={handleReset} hitSlop={8} style={styles.resetButton}>
            <RotateCcw color={colors.textSecondary} size={16} />
          </Pressable>
        )}
        {trailing}
      </View>
      <RNSlider
        style={styles.slider}
        minimumValue={min}
        maximumValue={max}
        step={step}
        value={value}
        minimumTrackTintColor={colors.accent}
        maximumTrackTintColor={colors.chip}
        thumbTintColor={colors.accent}
        onValueChange={handleChange}
        onSlidingComplete={handleSlidingComplete}
      />
    </View>
  );
}

function createStyles(colors: ThemeColors) {
  return StyleSheet.create({
    header: { flexDirection: 'row', alignItems: 'center', gap: 10 },
    label: { flex: 1, color: colors.textPrimary, fontSize: 14, fontWeight: '600' },
    value: { color: colors.accent, fontSize: 14, fontWeight: '700', minWidth: 44, textAlign: 'right' },
    slider: { height: 40, marginHorizontal: TRACK_EDGE_MARGIN },
    resetButton: {
      width: 28,
      height: 28,
      borderRadius: 14,
      backgroundColor: colors.chip,
      alignItems: 'center',
      justifyContent: 'center',
    },
  });
}
