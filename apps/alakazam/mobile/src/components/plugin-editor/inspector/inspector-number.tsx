import { pluginEditorItems, pluginEditorRebuild } from '@/src/state/plugin-editor';
import { memo, useEffect, useRef, useState } from 'react';
import { InspectorNumberField } from './inspector-number-field';
import { InspectorRow } from './inspector-row';

/**
 * One number a plugin put in its inspector, a field across the right of its name. Each number typed is held to `min` and `max`, and then
 * the plugin is told and the editor runs it again, since a number like this changes what the plugin makes and is not a drag. Like the
 * other controls it takes plain values and finds the plugin's current `onChange` in the state when the number changes.
 */
export const InspectorNumber = memo(function InspectorNumber({
  itemKey,
  label,
  value,
  min,
  max,
  step,
  reset,
}: {
  /** Names the control among the plugin's controls, which is how its current `onChange` is found. */
  itemKey: string;
  label: string;
  /** What the number is when the control shows. The control keeps its own number after that. */
  value: number;
  min?: number;
  max?: number;
  step: number;
  /** Shows a reset button beside the name. */
  reset: boolean;
}) {
  const [current, setCurrent] = useState(value);
  // The plugin can start the number somewhere else (it was given another value), and then the field shows it.
  useEffect(() => setCurrent(value), [value]);
  // What the number was when the control first showed, which is what the reset button puts it back to.
  const original = useRef(value);

  const set = (typed: number) => {
    const limited = Math.min(max ?? Infinity, Math.max(min ?? -Infinity, typed));
    setCurrent(limited);
    const item = pluginEditorItems.value.find(each => each.key === itemKey);
    if (item?.kind === 'number') item.onChange?.(limited);
    pluginEditorRebuild.next();
  };

  return (
    <InspectorRow label={label} onReset={reset ? () => set(original.current) : undefined}>
      <InspectorNumberField fill value={current} step={step} label={label} onChange={set} />
    </InspectorRow>
  );
});
