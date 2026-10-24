import { useTheme } from '@/src/lib/ui/theme';
import { Text, View } from 'react-native';

/** Labeled progress indicator for long-running tasks. `progress` is between 0 and 1. */
export function ProgressBar({ progress, label, detail }: { progress: number; label: string; detail?: string }) {
  const { colors } = useTheme();
  const boundedProgress = Math.max(0, Math.min(1, progress));
  const percent = Math.round(boundedProgress * 100);

  return (
    <View accessibilityRole="progressbar" accessibilityValue={{ min: 0, max: 100, now: percent }}>
      <View style={{ flexDirection: 'row', justifyContent: 'space-between', marginBottom: 6 }}>
        <Text style={{ color: colors.textSecondary, fontSize: 13 }}>{label}</Text>
        <Text style={{ color: colors.textSecondary, fontSize: 13 }}>{detail ?? `${percent}%`}</Text>
      </View>
      <View style={{ height: 6, borderRadius: 3, overflow: 'hidden', backgroundColor: colors.chip }}>
        <View style={{ height: '100%', width: `${percent}%`, backgroundColor: colors.accent }} />
      </View>
    </View>
  );
}
