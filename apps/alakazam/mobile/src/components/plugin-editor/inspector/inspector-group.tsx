import { useTheme } from '@/src/lib/ui/theme';
import { pluginEditorItems, pluginEditorRebuild } from '@/src/state/plugin-editor';
import { memo } from 'react';
import ChevronDown from 'lucide-react-native/icons/chevron-down';
import ChevronRight from 'lucide-react-native/icons/chevron-right';
import Square from 'lucide-react-native/icons/square';
import SquareCheck from 'lucide-react-native/icons/square-check';
import { Pressable, Text, View } from 'react-native';

/**
 * The title of a foldable group a plugin made. Tapping the title or its arrow shows or hides the controls after it, up to the next group.
 * A group that can be switched off as a whole has a checkbox before the title; the plugin is told when it is tapped and the editor runs it
 * again, like any control that changes what the plugin makes. The title of a group that is off is dimmed, and its controls stay so they can still be set.
 */
export const InspectorGroup = memo(function InspectorGroup({
  itemKey,
  text,
  open,
  enabled,
  first,
  onFold,
}: {
  /** Names the group among the plugin's controls, which is how its current `onToggle` is found. */
  itemKey: string;
  text: string;
  open: boolean;
  /** Whether the checkbox is ticked, or nothing for a group with no checkbox. */
  enabled?: boolean;
  /** The first thing in the inspector, so there is nothing above it to leave a gap from. */
  first: boolean;
  onFold: () => void;
}) {
  const { colors } = useTheme();
  const Chevron = open ? ChevronDown : ChevronRight;
  const Box = enabled ? SquareCheck : Square;
  const toggle = () => {
    const item = pluginEditorItems.value.find(each => each.key === itemKey);
    if (item?.kind === 'group') item.onToggle?.(!enabled);
    pluginEditorRebuild.next();
  };
  return (
    <View style={{ flexDirection: 'row', alignItems: 'center', gap: 6, marginTop: first ? 0 : 24 }}>
      {enabled === undefined ? null : (
        <Pressable accessibilityRole="checkbox" accessibilityLabel={`${text} on`} accessibilityState={{ checked: enabled }} hitSlop={8} onPress={toggle}>
          <Box color={enabled ? colors.accent : colors.textSecondary} size={20} />
        </Pressable>
      )}
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={text}
        accessibilityState={{ expanded: open }}
        hitSlop={8}
        onPress={onFold}
        style={{ flex: 1, flexDirection: 'row', alignItems: 'center', gap: 4 }}
      >
        <Chevron color={colors.accent} size={18} />
        <Text style={{ color: enabled === false ? colors.textSecondary : colors.textPrimary, fontWeight: '700' }}>{text}</Text>
      </Pressable>
    </View>
  );
});
