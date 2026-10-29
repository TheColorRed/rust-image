import { useTheme } from '@/src/lib/ui/theme';
import { pluginEditorItems, pluginEditorRebuild } from '@/src/state/plugin-editor';
import { memo, useEffect, useState } from 'react';
import { Pressable, StyleSheet, Text, View } from 'react-native';
import { InspectorRow } from './inspector-row';

/** Splits the choices, which come as one string so the control is only drawn again when they change. */
const SEPARATOR = '\u001f';

/** The choices as the one string `InspectorSelect` takes. */
export const joinChoices = (choices: string[]) => choices.join(SEPARATOR);

/**
 * A choice a plugin put in its inspector, as a bar of buttons with the chosen one filled, across the whole width under the name. A choice
 * changes what the plugin makes, so the plugin is told and the editor runs it again. Like the other controls it takes plain values and
 * finds the plugin's current `onChange` in the state when a choice is picked.
 */
export const InspectorSelect = memo(function InspectorSelect({
  itemKey,
  label,
  choices,
  value,
}: {
  /** Names the control among the plugin's controls, which is how its current `onChange` is found. */
  itemKey: string;
  label: string;
  /** The choices joined by `joinChoices`. */
  choices: string;
  /** The choice that is selected when the control shows. The control keeps its own choice after that. */
  value: string;
}) {
  const { colors } = useTheme();
  const [selected, setSelected] = useState(value);
  // The plugin can select another choice (it was given another value), and then the bar shows it.
  useEffect(() => setSelected(value), [value]);

  const pick = (choice: string) => {
    setSelected(choice);
    const item = pluginEditorItems.value.find(each => each.key === itemKey);
    if (item?.kind === 'select') item.onChange?.(choice);
    pluginEditorRebuild.next();
  };

  return (
    <InspectorRow label={label} stacked>
      <View style={[styles.bar, { borderColor: colors.border, backgroundColor: colors.chip }]}>
        {choices.split(SEPARATOR).map(choice => {
          const chosen = choice === selected;
          return (
            <Pressable
              key={choice}
              accessibilityRole="button"
              accessibilityState={{ selected: chosen }}
              onPress={() => pick(choice)}
              style={[styles.choice, chosen && { backgroundColor: colors.accent }]}
            >
              <Text numberOfLines={1} style={[styles.text, { color: chosen ? '#fff' : colors.textPrimary }]}>
                {choice}
              </Text>
            </Pressable>
          );
        })}
      </View>
    </InspectorRow>
  );
});

const styles = StyleSheet.create({
  bar: { flexDirection: 'row', borderWidth: StyleSheet.hairlineWidth, borderRadius: 6, overflow: 'hidden' },
  choice: { flex: 1, minHeight: 32, paddingHorizontal: 6, alignItems: 'center', justifyContent: 'center' },
  text: { fontSize: 13, fontWeight: '600' },
});
