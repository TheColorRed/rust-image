import { stopsOf } from '@/src/lib/color/gradient-math';
import { useTheme } from '@/src/lib/ui/theme';
import { pluginEditorItems, pluginEditorRebuild } from '@/src/state/plugin-editor';
import type { ColorSwatch } from '@/src/state/color-picker';
import { pickGradient, type GradientValue } from '@/src/state/gradient-picker';
import { memo, useEffect, useId, useState } from 'react';
import { Pressable, StyleSheet } from 'react-native';
import Svg, { Defs, LinearGradient, Pattern, Rect, Stop } from 'react-native-svg';
import { InspectorRow } from './inspector-row';

const HEIGHT = 28;

/**
 * A gradient a plugin put in its inspector: a small bar of the gradient (over a checkerboard, so how strong it is shows), which opens the gradient
 * editor. Closing the editor with OK changes what the plugin makes and is not a drag, so the plugin is told and the editor runs it again. Like the
 * other controls it takes plain values and finds the plugin's current `onChange` in the state when the editor is closed.
 */
export const InspectorGradient = memo(function InspectorGradient({
  itemKey,
  label,
  value,
  title,
  swatches,
}: {
  /** Names the control among the plugin's controls, which is how its current handlers are found. */
  itemKey: string;
  label: string;
  /** The gradient when the control shows. The control keeps its own gradient after that. */
  value: GradientValue;
  /** The words over the editor. Defaults to the control's name. */
  title?: string;
  swatches?: ColorSwatch[];
}) {
  const { colors } = useTheme();
  const [gradient, setGradient] = useState(value);
  // The plugin can start the control with another gradient (it was given another value), and then the control shows it.
  useEffect(() => setGradient(value), [value]);
  // Each bar needs ids of its own, since a picture can hold several.
  const id = useId().replace(/[^a-zA-Z0-9]/g, '');

  const open = async () => {
    const plugin = () => {
      const item = pluginEditorItems.value.find(each => each.key === itemKey);
      return item?.kind === 'gradient' ? item : undefined;
    };
    const first = gradient;
    const picked = await pickGradient({
      title: title ?? label,
      value: gradient,
      swatches,
      // The plugin shows the gradient as it is edited.
      onChange: live => plugin()?.onPreview?.(live),
    });
    if (!picked) {
      // Cancelled: the plugin takes back what it showed.
      plugin()?.onPreview?.(first);
      return;
    }
    setGradient(picked);
    plugin()?.onChange?.(picked);
    pluginEditorRebuild.next();
  };

  return (
    <InspectorRow label={label}>
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={`${label}: gradient`}
        onPress={open}
        style={[styles.button, { borderColor: colors.border }]}
      >
        <Svg width="100%" height={HEIGHT} pointerEvents="none">
          <Defs>
            <Pattern id={`${id}c`} x="0" y="0" width="12" height="12" patternUnits="userSpaceOnUse">
              <Rect width="12" height="12" fill="#ffffff" />
              <Rect width="6" height="6" fill="#c8c8c8" />
              <Rect x="6" y="6" width="6" height="6" fill="#c8c8c8" />
            </Pattern>
            <LinearGradient id={`${id}g`} x1="0" y1="0" x2="1" y2="0">
              {stopsOf(gradient).map((stop, index) => (
                <Stop key={index} offset={String(stop.at)} stopColor={`rgb(${stop.rgb.join(', ')})`} stopOpacity={String(stop.alpha / 255)} />
              ))}
            </LinearGradient>
          </Defs>
          <Rect width="100%" height={HEIGHT} fill={`url(#${id}c)`} />
          <Rect width="100%" height={HEIGHT} fill={`url(#${id}g)`} />
        </Svg>
      </Pressable>
    </InspectorRow>
  );
});

const styles = StyleSheet.create({
  button: { borderWidth: 1, borderRadius: 6, overflow: 'hidden' },
});
