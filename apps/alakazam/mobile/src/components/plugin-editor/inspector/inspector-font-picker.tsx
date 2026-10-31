import { FontBrowser } from '@/src/components/font-browser';
import { useTheme } from '@/src/lib/ui/theme';
import { pluginEditorItems, pluginEditorRebuild } from '@/src/state/plugin-editor';
import { memo, useEffect, useState } from 'react';
import { Modal, Pressable, StyleSheet, Text } from 'react-native';
import { InspectorRow } from './inspector-row';

/**
 * A font chooser a plugin put in its inspector: a button showing the chosen family that opens the Fonts overlay, where the fonts on the
 * device are browsed and more are downloaded. Like the other controls it takes plain values and finds the plugin's current `onPick` in
 * the state when a font is picked.
 */
export const InspectorFontPicker = memo(function InspectorFontPicker({
  itemKey,
  label,
  value,
}: {
  /** Names the control among the plugin's controls, which is how its current `onPick` is found. */
  itemKey: string;
  label: string;
  /** The family chosen when the control shows. The control keeps its own choice after that. */
  value: string | undefined;
}) {
  const { colors } = useTheme();
  const [family, setFamily] = useState(value);
  const [open, setOpen] = useState(false);
  // The plugin can start the control with another family (it was given another value), and then the button shows it.
  useEffect(() => setFamily(value), [value]);

  const pick = (picked: string) => {
    setFamily(picked);
    setOpen(false);
    const item = pluginEditorItems.value.find(each => each.key === itemKey);
    if (item?.kind === 'font-picker') item.onPick?.(picked);
    pluginEditorRebuild.next();
  };

  return (
    <InspectorRow label={label}>
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={`${label}: ${family ?? 'none chosen'}`}
        onPress={() => setOpen(true)}
        style={[styles.button, { backgroundColor: colors.chip, borderColor: colors.border }]}
      >
        <Text numberOfLines={1} style={{ color: family ? colors.textPrimary : colors.textSecondary, fontSize: 14 }}>
          {family ?? 'Choose a font'}
        </Text>
      </Pressable>
      <Modal visible={open} animationType="slide" onRequestClose={() => setOpen(false)}>
        <FontBrowser selected={family} onPick={pick} onClose={() => setOpen(false)} />
      </Modal>
    </InspectorRow>
  );
});

const styles = StyleSheet.create({
  button: { borderWidth: 1, borderRadius: 8, paddingHorizontal: 10, paddingVertical: 6 },
});
