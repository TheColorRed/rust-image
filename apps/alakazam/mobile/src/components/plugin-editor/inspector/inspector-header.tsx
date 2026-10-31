import type { PluginHeaderLevel } from '@/src/lib/plugins/plugin-inspector';
import { useTheme } from '@/src/lib/ui/theme';
import { memo } from 'react';
import ChevronLeft from 'lucide-react-native/icons/chevron-left';
import { Pressable, Text } from 'react-native';

/** The text size of each heading level, from <h1> down to <h6>. */
const HEADER_SIZES = { 1: 24, 2: 20, 3: 18, 4: 16, 5: 14, 6: 12 } as const;

/** A heading a plugin put in its inspector. */
export const InspectorHeader = memo(function InspectorHeader({
  text,
  level,
  first,
  onBack,
}: {
  text: string;
  /** Shows a Back button before the heading, which calls this. */
  onBack?: () => void;
  level: PluginHeaderLevel;
  /** The first thing in the inspector, so there is nothing above it to leave a gap from. */
  first: boolean;
}) {
  const { colors } = useTheme();
  if (onBack) {
    return (
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={`Back from ${text}`}
        hitSlop={8}
        onPress={onBack}
        style={{ flexDirection: 'row', alignItems: 'center', gap: 4, marginTop: first ? 0 : 36, marginBottom: -6 }}
      >
        <ChevronLeft color={colors.accent} size={18} />
        <Text accessibilityRole="header" style={{ color: colors.textPrimary, fontWeight: '700' }}>
          {text}
        </Text>
      </Pressable>
    );
  }
  return (
    <Text
      accessibilityRole="header"
      style={{ color: colors.textPrimary, fontWeight: '700', marginTop: first ? 0 : 36, marginBottom: -6 /*  fontSize: HEADER_SIZES[level] */ }}
    >
      {text}
    </Text>
  );
});
