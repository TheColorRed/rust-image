import { useTheme } from '@/src/lib/ui/theme';
import RotateCcw from 'lucide-react-native/icons/rotate-ccw';
import type { ReactNode } from 'react';
import { Pressable, StyleSheet, Text, View } from 'react-native';

/**
 * One line of the inspector: the control's name on the left and the control on the right. Every row gives the name the same share of
 * the width, so the controls of all the rows start at the same place and are the same width, the way the Unity inspector lines them up.
 */
export function InspectorRow({
  label,
  labelOffset = 0,
  stacked = false,
  onReset,
  children,
}: {
  label: string;
  /** Moves the name down, for a control that has a small label above the part you use, so the name lines up with that part. */
  labelOffset?: number;
  /** Puts the name on a line of its own with the control under it, across the whole width. */
  stacked?: boolean;
  /** Shows a reset button, a round icon, beside the name, which calls this. Left out when the control has nothing to reset. */
  onReset?: () => void;
  children: ReactNode;
}) {
  const { colors } = useTheme();
  return (
    <View style={stacked ? styles.stacked : styles.row}>
      <View style={[stacked ? styles.stackedName : styles.name, { marginTop: labelOffset }]}>
        <Text numberOfLines={2} style={[styles.label, { color: colors.textPrimary }]}>
          {label}
        </Text>
        {onReset ? (
          <Pressable
            accessibilityRole="button"
            accessibilityLabel={`Reset ${label}`}
            onPress={onReset}
            hitSlop={8}
            style={[styles.reset, { backgroundColor: colors.chip }]}
          >
            <RotateCcw color={colors.textSecondary} size={12} />
          </Pressable>
        ) : null}
      </View>
      <View style={stacked ? styles.stackedControl : styles.control}>{children}</View>
    </View>
  );
}

const styles = StyleSheet.create({
  row: { flexDirection: 'row', alignItems: 'flex-start', gap: 8 },
  name: { width: '36%', minHeight: 36, flexDirection: 'row', alignItems: 'center', gap: 6 },
  control: { flex: 1, minWidth: 0 },
  stacked: { gap: 2 },
  stackedName: { minHeight: 0, flexDirection: 'row', alignItems: 'center', gap: 6 },
  stackedControl: { minWidth: 0 },
  label: { flexShrink: 1, fontSize: 14, fontWeight: '600' },
  reset: { width: 22, height: 22, borderRadius: 11, alignItems: 'center', justifyContent: 'center' },
});
