import { useTheme } from '@/src/lib/ui/theme';
import { pluginEditorItems, pluginEditorRebuild } from '@/src/state/plugin-editor';
import { memo } from 'react';
import ChevronRight from 'lucide-react-native/icons/chevron-right';
import { Pressable, Text } from 'react-native';

/**
 * A row a plugin put in its inspector that opens a submenu: the name on the left and an arrow on the right. Tapping it tells the plugin, and the
 * editor runs the plugin again so it can show the submenu's controls in place of these.
 */
export const InspectorSubmenu = memo(function InspectorSubmenu({ itemKey, text }: { itemKey: string; text: string }) {
  const { colors } = useTheme();
  const open = () => {
    const item = pluginEditorItems.value.find(each => each.key === itemKey);
    if (item?.kind === 'submenu') item.onOpen?.();
    pluginEditorRebuild.next();
  };
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={text}
      hitSlop={8}
      onPress={open}
      style={{ flexDirection: 'row', alignItems: 'center', justifyContent: 'space-between', minHeight: 32 }}
    >
      <Text style={{ color: colors.textPrimary }}>{text}</Text>
      <ChevronRight color={colors.accent} size={18} />
    </Pressable>
  );
});
