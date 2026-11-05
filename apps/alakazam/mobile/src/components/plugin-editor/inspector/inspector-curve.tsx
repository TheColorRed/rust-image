import { curveSamples } from '@/src/lib/curve/curve-samples';
import { useTheme } from '@/src/lib/ui/theme';
import { pickCurve, type CurvePreset, type CurveValue } from '@/src/state/curve-picker';
import { pluginEditorItems, pluginEditorRebuild } from '@/src/state/plugin-editor';
import { memo, useEffect, useMemo, useState } from 'react';
import { Pressable, StyleSheet } from 'react-native';
import Svg, { Path } from 'react-native-svg';
import { InspectorRow } from './inspector-row';

const HEIGHT = 44;
const SAMPLES = 40;
/** The drawing is on a fixed scale that stretches to the width the row has. */
const VIEW_WIDTH = 100;

/**
 * A curve a plugin put in its inspector: a small drawing of the curve across the width of the row, which opens the curve editor. Closing the editor
 * with OK changes what the plugin makes and is not a drag, so the plugin is told and the editor runs it again. Like the other controls it takes
 * plain values and finds the plugin's current `onChange` in the state when the editor is closed.
 */
export const InspectorCurve = memo(function InspectorCurve({
  itemKey,
  label,
  value,
  min,
  max,
  title,
  presets,
}: {
  /** Names the control among the plugin's controls, which is how its current handlers are found. */
  itemKey: string;
  label: string;
  /** The curve when the control shows. The control keeps its own curve after that. */
  value: CurveValue;
  min: number;
  max: number;
  title?: string;
  presets?: CurvePreset[];
}) {
  const { colors } = useTheme();
  const [curve, setCurve] = useState(value);
  // The plugin can start the control with another curve (it was given another value), and then the control shows it.
  useEffect(() => setCurve(value), [value]);

  const line = useMemo(() => {
    const span = max > min ? max - min : 1;
    return curveSamples(curve, SAMPLES)
      .map((sample, at) => {
        const height = Math.min(Math.max((sample - min) / span, 0), 1);
        return `${at === 0 ? 'M' : 'L'}${((at / SAMPLES) * VIEW_WIDTH).toFixed(1)},${(HEIGHT - 4 - height * (HEIGHT - 8)).toFixed(1)}`;
      })
      .join(' ');
  }, [curve, min, max]);

  const open = async () => {
    const plugin = () => {
      const item = pluginEditorItems.value.find(each => each.key === itemKey);
      return item?.kind === 'curve' ? item : undefined;
    };
    const first = curve;
    const picked = await pickCurve({
      title: title ?? label,
      value: curve,
      min,
      max,
      presets,
      // The plugin shows the curve as it is edited.
      onChange: live => plugin()?.onPreview?.(live),
    });
    if (!picked) {
      // Cancelled: the plugin takes back what it showed.
      plugin()?.onPreview?.(first);
      return;
    }
    setCurve(picked);
    plugin()?.onChange?.(picked);
    pluginEditorRebuild.next();
  };

  return (
    <InspectorRow label={label}>
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={`${label}: curve`}
        onPress={open}
        style={[styles.button, { backgroundColor: colors.chip, borderColor: colors.border }]}
      >
        <Svg width="100%" height={HEIGHT} viewBox={`0 0 ${VIEW_WIDTH} ${HEIGHT}`} preserveAspectRatio="none" pointerEvents="none">
          <Path d={line} fill="none" stroke={colors.accent} strokeWidth={2} vectorEffect="non-scaling-stroke" />
        </Svg>
      </Pressable>
    </InspectorRow>
  );
});

const styles = StyleSheet.create({
  button: { borderWidth: 1, borderRadius: 6, overflow: 'hidden' },
});
