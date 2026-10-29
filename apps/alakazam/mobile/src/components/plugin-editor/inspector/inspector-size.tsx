import { useTheme } from '@/src/lib/ui/theme';
import { pluginEditorItems, pluginEditorRebuild } from '@/src/state/plugin-editor';
import { memo, useEffect, useRef, useState } from 'react';
import { StyleSheet, Text, View } from 'react-native';
import { InspectorNumberField } from './inspector-number-field';
import { InspectorRow } from './inspector-row';

/** How far the row's own name is moved down so it lines up with the fields: past the small name above each one (14 high) and a little
 * more, since the row centres its name in a taller line than the field is. */
const FIELD_LABEL_HEIGHT = 14;

/**
 * Two numbers a plugin put in its inspector, side by side, each with a label above it: a size (width and height) or a point (x and
 * y). Each is an `InspectorNumberField`, so a field can be emptied to type a new number and the plugin hears only real numbers. The plugin is told
 * with both numbers, and the editor runs the plugin again, since a size is a change to what the plugin makes and not a drag. Like the
 * other controls it takes plain values and finds the plugin's current `onChange` in the state when a number changes.
 */
export const InspectorSize = memo(function InspectorSize({
  itemKey,
  label,
  labels,
  x,
  y,
  reset,
  min,
  max,
}: {
  /** Names the control among the plugin's controls, which is how its current `onChange` is found. */
  itemKey: string;
  label: string;
  labels: [string, string];
  /** What the two numbers start as. The fields keep their own numbers after that. */
  x: number;
  y: number;
  /** Shows a reset button beside the name. */
  reset: boolean;
  min?: number;
  max?: number;
}) {
  const { colors } = useTheme();
  const [current, setCurrent] = useState<[number, number]>([x, y]);
  // The plugin can start the numbers somewhere else (it was given other values), and then the fields show them.
  useEffect(() => setCurrent([x, y]), [x, y]);

  // What the numbers were when the control first showed, which is what the reset button puts them back to.
  const original = useRef<[number, number]>([x, y]);

  /** Keeps the numbers, tells the plugin and has it run again. Each number is held to `min` and `max`. */
  const set = (next: [number, number]) => {
    const limited = next.map(each => Math.min(max ?? Infinity, Math.max(min ?? -Infinity, each))) as [number, number];
    setCurrent(limited);
    const item = pluginEditorItems.value.find(each => each.key === itemKey);
    if (item?.kind === 'size') item.onChange?.(limited);
    pluginEditorRebuild.next();
  };

  const field = (at: 0 | 1) => (
    <View style={styles.field}>
      <Text numberOfLines={1} style={[styles.fieldLabel, { color: colors.textSecondary }]}>
        {labels[at]}
      </Text>
      <InspectorNumberField
        fill
        value={current[at]}
        step={1}
        label={`${label} ${labels[at]}`}
        onChange={typed => set(at === 0 ? [typed, current[1]] : [current[0], typed])}
      />
    </View>
  );

  return (
    <InspectorRow label={label} labelOffset={FIELD_LABEL_HEIGHT} onReset={reset ? () => set(original.current) : undefined}>
      <View style={styles.fields}>
        {field(0)}
        {field(1)}
      </View>
    </InspectorRow>
  );
});

const styles = StyleSheet.create({
  fields: { flexDirection: 'row', gap: 8 },
  field: { flex: 1, gap: 2, minWidth: 0 },
  fieldLabel: { fontSize: 12, height: 14, textAlign: 'left' },
});
