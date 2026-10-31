import { useTheme } from '@/src/lib/ui/theme';
import { pluginEditorItems, pluginEditorRebuild } from '@/src/state/plugin-editor';
import { memo, useEffect, useState } from 'react';
import { Switch } from 'react-native';
import { InspectorRow } from './inspector-row';

/**
 * An on/off switch a plugin put in its inspector, on the right of its name. Turning it changes what the plugin makes and is not a drag,
 * so the plugin is told and the editor runs it again. Like the other controls it takes plain values and finds the plugin's current
 * `onChange` in the state when the switch is turned.
 */
export const InspectorToggle = memo(function InspectorToggle({
  itemKey,
  label,
  value,
}: {
  /** Names the control among the plugin's controls, which is how its current `onChange` is found. */
  itemKey: string;
  label: string;
  /** Whether the switch is on when the control shows. The control keeps its own state after that. */
  value: boolean;
}) {
  const { colors } = useTheme();
  const [on, setOn] = useState(value);
  // The plugin can start the switch somewhere else (it was given another value), and then the switch shows it.
  useEffect(() => setOn(value), [value]);

  const toggle = (next: boolean) => {
    setOn(next);
    const item = pluginEditorItems.value.find(each => each.key === itemKey);
    if (item?.kind === 'toggle') item.onChange?.(next);
    pluginEditorRebuild.next();
  };

  return (
    <InspectorRow label={label}>
      <Switch
        accessibilityLabel={label}
        value={on}
        onValueChange={toggle}
        trackColor={{ false: colors.chip, true: colors.accent }}
        thumbColor="#fff"
        // Scaled about its left edge, so it stays lined up with the controls above and below it. A transform does not change layout, so the
        // switch still takes the height of a full-size one (about 28 dp) and the negative margin gives back what the scaling took.
        style={{ alignSelf: 'flex-start', transform: [{ scale: 0.8 }], transformOrigin: 'left center', marginVertical: -3 }}
      />
    </InspectorRow>
  );
});
