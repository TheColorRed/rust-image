import { RangeSlider } from '@/src/components/range-slider';
import { Slider } from '@/src/components/slider';
import { pluginEditorItems, pluginEditorRedraw } from '@/src/state/plugin-editor';
import { memo, useEffect, useMemo, useRef, useState } from 'react';
import { StyleSheet, View } from 'react-native';
import { InspectorNumberField } from './inspector-number-field';
import { InspectorRow } from './inspector-row';

/**
 * A slider a plugin put in its inspector, with one thumb or two, and a number to type at the end of it (at both ends for two
 * thumbs), as in the Unity inspector. It takes only plain values, so it is drawn again only when the slider itself changes, and it
 * finds the plugin's `onChange` in the state when the slider moves, because the plugin makes a new one each time its inspector is made.
 */
export const InspectorSlider = memo(function InspectorSlider({
  itemKey,
  label,
  min,
  max,
  step,
  range,
  multiline,
  reset,
  value,
}: {
  /** Names the slider among the plugin's controls, which is how its current `onChange` is found. */
  itemKey: string;
  label: string;
  min: number;
  max: number;
  step: number;
  range: boolean;
  /** The name is on a line of its own and the slider is under it, across the whole width. */
  multiline: boolean;
  /** Shows a reset button beside the name. */
  reset: boolean;
  /** Where the thumb or thumbs start. */
  value: number | [number, number];
}) {
  const start: [number, number] = Array.isArray(value) ? value : [value, value];
  const [current, setCurrent] = useState<[number, number]>(start);
  // Where the slider was when it first showed, which is what the reset button puts it back to.
  const original = useRef<[number, number]>(start);
  // The plugin can start the slider somewhere else (it was given other values), and then it moves there.
  useEffect(() => setCurrent([start[0], start[1]]), [start[0], start[1]]);
  const bounds = useMemo(
    () => ({ min: [min, min] as [number, number], max: [max, max] as [number, number], step: [step, step] as [number, number] }),
    [min, max, step],
  );

  /** The slider was moved or a number was typed: keeps it, tells the plugin and has the preview drawn again. */
  const move = (next: [number, number]) => {
    setCurrent(next);
    const item = pluginEditorItems.value.find(each => each.key === itemKey);
    if (item?.kind === 'slider') item.onChange?.(range ? next : next[0]);
    pluginEditorRedraw.next();
  };
  const limit = (typed: number) => Math.min(max, Math.max(min, typed));

  // The name is the row's, and the numbers are the fields at the ends, so the slider draws only its track.
  return (
    <InspectorRow label={label} stacked={multiline} onReset={reset ? () => move(original.current) : undefined}>
      <View style={styles.line}>
        {range ? (
          <InspectorNumberField
            value={current[0]}
            step={step}
            label={`${label} low`}
            onChange={typed => move([Math.min(limit(typed), current[1]), current[1]])}
          />
        ) : null}
        <View style={styles.track}>
          {range ? (
            <RangeSlider
              label=""
              compact
              min={bounds.min}
              max={bounds.max}
              step={bounds.step}
              initialValue={current}
              reset={false}
              onChange={move}
            />
          ) : (
            <Slider
              label=""
              compact
              min={min}
              max={max}
              step={step}
              initialValue={current[0]}
              reset={false}
              onChange={next => move([next, next])}
            />
          )}
        </View>
        <InspectorNumberField
          value={range ? current[1] : current[0]}
          step={step}
          label={range ? `${label} high` : label}
          onChange={typed => move(range ? [current[0], Math.max(limit(typed), current[0])] : [limit(typed), limit(typed)])}
        />
      </View>
    </InspectorRow>
  );
});

const styles = StyleSheet.create({
  line: { flexDirection: 'row', alignItems: 'center', gap: 6 },
  track: { flex: 1, minWidth: 0 },
});
