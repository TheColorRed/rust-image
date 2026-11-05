import { rgbToHex, type Rgb } from '@/src/lib/color/color-math';
import { useTheme } from '@/src/lib/ui/theme';
import { pluginEditorItems, pluginEditorRebuild } from '@/src/state/plugin-editor';
import { pickColor, type ColorSwatch } from '@/src/state/color-picker';
import { memo, useEffect, useState } from 'react';
import { Pressable, StyleSheet, Text, View } from 'react-native';
import { InspectorRow } from './inspector-row';

/**
 * A color a plugin put in its inspector: a swatch of the color with its web color beside it, which opens the color picker. A pick changes what
 * the plugin makes and is not a drag, so the plugin is told and the editor runs it again. Like the other controls it takes plain values and
 * finds the plugin's current `onChange` in the state when a color is picked.
 */
export const InspectorColor = memo(function InspectorColor({
  itemKey,
  label,
  value,
  alpha,
  title,
  swatches,
}: {
  /** Names the control among the plugin's controls, which is how its current `onChange` is found. */
  itemKey: string;
  label: string;
  /** The color when the control shows: `[red, green, blue]`, with the strength as a fourth number when `alpha` is true. The control keeps its own color after that. */
  value: number[];
  /** Whether the color has a strength (a fourth number, 0 to 255). */
  alpha: boolean;
  /** The words over the picker. Defaults to the control's name. */
  title?: string;
  /** The named colors the picker shows, or the app's common set. */
  swatches?: ColorSwatch[];
}) {
  const { colors } = useTheme();
  const [color, setColor] = useState(value);
  // The plugin can start the control with another color (it was given another value), and then the control shows it.
  useEffect(() => setColor(value), [value]);
  const rgb = color.slice(0, 3) as Rgb;
  const strength = alpha ? (color[3] ?? 255) : 255;

  const open = async () => {
    /** The plugin's current handlers for this control, found in the state each time, as the other controls do. */
    const plugin = () => {
      const item = pluginEditorItems.value.find(each => each.key === itemKey);
      return item?.kind === 'color' ? item : undefined;
    };
    const formatted = (picked: { rgb: Rgb; alpha: number }) => (alpha ? [...picked.rgb, picked.alpha] : [...picked.rgb]);
    const first = formatted({ rgb, alpha: strength });
    const picked = await pickColor({
      title: title ?? label,
      value: rgb,
      alpha: alpha ? strength : undefined,
      swatches,
      // The plugin shows the color as it is dragged.
      onChange: live => plugin()?.onPreview?.(formatted(live)),
    });
    if (!picked) {
      // Cancelled: the plugin takes back what it showed.
      plugin()?.onPreview?.(first);
      return;
    }
    const next = formatted(picked);
    setColor(next);
    plugin()?.onChange?.(next);
    pluginEditorRebuild.next();
  };

  return (
    <InspectorRow label={label}>
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={`${label}: ${rgbToHex(rgb)}`}
        onPress={open}
        style={[styles.button, { backgroundColor: colors.chip, borderColor: colors.border }]}
      >
        <View style={[styles.swatch, { borderColor: colors.border }]}>
          <View style={[StyleSheet.absoluteFill, { backgroundColor: `rgb(${rgb.join(', ')})`, opacity: strength / 255 }]} />
        </View>
        <Text numberOfLines={1} style={{ color: colors.textPrimary, fontSize: 14 }}>
          {rgbToHex(rgb)}
        </Text>
        {alpha ? <Text style={{ color: colors.textSecondary, fontSize: 12 }}>{Math.round((strength / 255) * 100)}%</Text> : null}
      </Pressable>
    </InspectorRow>
  );
});

const styles = StyleSheet.create({
  button: { flexDirection: 'row', alignItems: 'center', gap: 8, borderWidth: 1, borderRadius: 8, paddingHorizontal: 8, paddingVertical: 5, alignSelf: 'flex-start' },
  swatch: { width: 26, height: 26, borderRadius: 6, borderWidth: 1, overflow: 'hidden', backgroundColor: '#ffffff' },
});
