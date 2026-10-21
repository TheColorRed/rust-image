import { RotateCcw } from 'lucide-react-native';
import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { PanResponder, Pressable, StyleSheet, Text, View } from 'react-native';
import { useTheme, type ThemeColors } from '@/src/lib/ui/theme';

/** The same room kept beside the track as the plain slider, so a drag stops short of the edge where the back swipe begins. */
const TRACK_EDGE_MARGIN = 20;
const THUMB_SIZE = 12;
const TRACK_HEIGHT = 2;

export interface RangeSliderProps {
  /** The lowest value of each thumb: the low thumb's, then the high thumb's. */
  min: [number, number];
  /** The highest value of each thumb: the low thumb's, then the high thumb's. */
  max: [number, number];
  /** The step between selectable values, for each thumb. */
  step?: [number, number];
  /** The label for the slider. */
  label: string;
  /** The low and the high value the slider starts at. */
  initialValue: [number, number];
  /** What the reset button returns the slider to. */
  resetValue?: [number, number];
  /** Whether the slider should show a reset button. */
  reset?: boolean;
  /** Called with the low and high value on every move. */
  onChange?: (value: [number, number]) => void;
  /** Called with the low and high value when a thumb is let go, and after a reset. */
  onTrigger?: (value: [number, number]) => void;
  /** Words to show in place of a number, as `Slider` has: it gets a value and returns the text, or nothing for the number. */
  valueText?: (value: number) => string | undefined;
  /** Shown at the start of the header line, before the label. */
  leading?: ReactNode;
  /** Shown at the end of the header line, after the reset button. */
  trailing?: ReactNode;
}

const snap = (value: number, min: number, max: number, step: number) => {
  const stepped = min + Math.round((value - min) / step) * step;
  return Math.min(max, Math.max(min, Number(stepped.toFixed(6))));
};

/**
 * A labelled slider with two thumbs, which pick a low and a high value on the same track. The low thumb never passes the high
 * one. The header has the label and the two values, as `Slider`'s does.
 */
export function RangeSlider({
  min,
  max,
  step = [1, 1],
  label,
  initialValue,
  resetValue = initialValue,
  reset = true,
  onChange,
  onTrigger,
  valueText,
  leading,
  trailing,
}: RangeSliderProps) {
  const { colors } = useTheme();
  const styles = useMemo(() => createStyles(colors), [colors]);
  const [value, setValue] = useState<[number, number]>(initialValue);
  const [width, setWidth] = useState(0);
  // What the responders read, since they are made once and would otherwise keep the first render's values.
  const live = useRef({ value, width, min, max, step, onChange, onTrigger });
  live.current = { value, width, min, max, step, onChange, onTrigger };
  const start = useRef<[number, number]>(value);
  // The newest value, which a release reads before React has drawn the last move.
  const latest = useRef<[number, number]>(value);

  // The track runs from the lowest to the highest value either thumb can take.
  const lowest = Math.min(min[0], min[1]);
  const highest = Math.max(max[0], max[1]);
  const toX = (v: number, w: number) => (highest > lowest ? ((v - lowest) / (highest - lowest)) * w : 0);

  useEffect(() => {
    latest.current = initialValue;
    setValue(initialValue);
  }, [initialValue[0], initialValue[1]]);

  const responder = (thumb: 0 | 1) =>
    PanResponder.create({
      onStartShouldSetPanResponder: () => true,
      onMoveShouldSetPanResponder: () => true,
      // Nothing else on the screen should take the drag from a thumb once it is held.
      onPanResponderTerminationRequest: () => false,
      onPanResponderGrant: () => {
        start.current = latest.current;
      },
      onPanResponderMove: (_event, gesture) => {
        const { width: w, min: lo, max: hi, step: st, onChange: change } = live.current;
        if (w <= 0) return;
        const next: [number, number] = [...start.current];
        const raw = start.current[thumb] + (gesture.dx / w) * (highest - lowest);
        next[thumb] = snap(raw, lo[thumb], hi[thumb], st[thumb]);
        // The thumbs may touch but not cross.
        if (thumb === 0) next[0] = Math.min(next[0], next[1]);
        else next[1] = Math.max(next[1], next[0]);
        latest.current = next;
        setValue(next);
        change?.(next);
      },
      onPanResponderRelease: () => live.current.onTrigger?.(latest.current),
      onPanResponderTerminate: () => live.current.onTrigger?.(latest.current),
    });
  const lowResponder = useMemo(() => responder(0), []);
  const highResponder = useMemo(() => responder(1), []);

  const text = (v: number) => valueText?.(v) ?? String(Number(v.toFixed(2)));
  const trackWidth = Math.max(0, width);
  const lowX = toX(value[0], trackWidth);
  const highX = toX(value[1], trackWidth);

  const handleReset = () => {
    latest.current = resetValue;
    setValue(resetValue);
    onChange?.(resetValue);
    onTrigger?.(resetValue);
  };

  return (
    <View>
      <View style={styles.header}>
        {leading}
        <Text style={styles.label} numberOfLines={1}>
          {label}
        </Text>
        <Text style={styles.value}>{`${text(value[0])} – ${text(value[1])}`}</Text>
        {reset && (
          <Pressable onPress={handleReset} hitSlop={8} style={styles.resetButton}>
            <RotateCcw color={colors.textSecondary} size={16} />
          </Pressable>
        )}
        {trailing}
      </View>
      <View style={styles.area}>
        <View style={styles.track} onLayout={event => setWidth(event.nativeEvent.layout.width)}>
          <View style={[styles.selected, { left: lowX, width: Math.max(0, highX - lowX) }]} />
          <View
            {...lowResponder.panHandlers}
            hitSlop={16}
            style={[styles.thumb, { left: lowX - THUMB_SIZE / 2 }]}
            accessibilityRole="adjustable"
            accessibilityLabel={`${label} low`}
          />
          <View
            {...highResponder.panHandlers}
            hitSlop={16}
            style={[styles.thumb, { left: highX - THUMB_SIZE / 2 }]}
            accessibilityRole="adjustable"
            accessibilityLabel={`${label} high`}
          />
        </View>
      </View>
    </View>
  );
}

function createStyles(colors: ThemeColors) {
  return StyleSheet.create({
    header: { flexDirection: 'row', alignItems: 'center', gap: 10 },
    label: { flex: 1, color: colors.textPrimary, fontSize: 14, fontWeight: '600' },
    value: { color: colors.accent, fontSize: 14, fontWeight: '700', minWidth: 44, textAlign: 'right' },
    area: { height: 40, justifyContent: 'center', marginHorizontal: TRACK_EDGE_MARGIN + THUMB_SIZE / 2 },
    track: { height: TRACK_HEIGHT, borderRadius: TRACK_HEIGHT / 2, backgroundColor: colors.chip },
    selected: { position: 'absolute', height: TRACK_HEIGHT, borderRadius: TRACK_HEIGHT / 2, backgroundColor: colors.accent },
    thumb: {
      position: 'absolute',
      top: (TRACK_HEIGHT - THUMB_SIZE) / 2,
      width: THUMB_SIZE,
      height: THUMB_SIZE,
      borderRadius: THUMB_SIZE / 2,
      backgroundColor: colors.accent,
    },
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
