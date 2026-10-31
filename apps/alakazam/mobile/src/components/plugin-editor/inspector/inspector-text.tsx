import { useTheme } from '@/src/lib/ui/theme';
import { pluginEditorItems, pluginEditorRebuild } from '@/src/state/plugin-editor';
import { memo, useEffect, useRef, useState } from 'react';
import { StyleSheet, TextInput } from 'react-native';
import { InspectorRow } from './inspector-row';

/** How long typing has to stop before the plugin is told, so it is not run again for every letter. */
const SETTLE_MS = 300;

/**
 * A line of text a plugin put in its inspector, a box across the right of its name. The plugin is told a moment after typing stops, and
 * then the editor runs it again. Like the other controls it takes plain values and finds the plugin's current `onChange` in the state.
 */
export const InspectorText = memo(function InspectorText({
  itemKey,
  label,
  value,
  placeholder,
}: {
  /** Names the control among the plugin's controls, which is how its current `onChange` is found. */
  itemKey: string;
  label: string;
  /** What the box holds when the control shows. The control keeps its own text after that. */
  value: string;
  placeholder?: string;
}) {
  const { colors } = useTheme();
  const [text, setText] = useState(value);
  // The plugin can start the box with something else (it was given another value), and then the box shows it.
  useEffect(() => setText(value), [value]);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  useEffect(() => () => clearTimeout(timer.current), []);

  const change = (typed: string) => {
    setText(typed);
    clearTimeout(timer.current);
    timer.current = setTimeout(() => {
      const item = pluginEditorItems.value.find(each => each.key === itemKey);
      if (item?.kind === 'text') item.onChange?.(typed);
      pluginEditorRebuild.next();
    }, SETTLE_MS);
  };

  return (
    <InspectorRow label={label}>
      <TextInput
        accessibilityLabel={label}
        value={text}
        onChangeText={change}
        placeholder={placeholder}
        placeholderTextColor={colors.textSecondary}
        autoCorrect={false}
        style={[styles.input, { color: colors.textPrimary, borderColor: colors.border }]}
      />
    </InspectorRow>
  );
});

const styles = StyleSheet.create({
  input: { borderWidth: 1, borderRadius: 8, paddingHorizontal: 10, paddingVertical: 6, fontSize: 14 },
});
