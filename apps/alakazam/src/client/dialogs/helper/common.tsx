import { useNumericInputValidation } from '@/client/hooks/numeric-input-validation';
import { useToggleAdjustments } from '@/client/hooks/toggle-adjustments';
import { NumericInputValue } from '@/client/lib/util';
import { Button } from '@/client/ui/button';
import { Input } from '@/client/ui/input';
import { Option } from '@/client/ui/option';
import { Select } from '@/client/ui/select';
import Slider from '@/client/ui/slider';
import { useEffect, useMemo, useRef, useState } from 'react';
import { useDebounce } from 'use-debounce';

export interface AdjustmentRowProps<T = number> {
  /** Label for the adjustment (e.g., "Brightness") */
  label: string;
  /** Current value of the adjustment. Defaults to 0. */
  value?: T;
  /** Options for discrete adjustments (e.g., dropdown). */
  options?: T[];
  /** Minimum value for the adjustment. Defaults to -Infinity. */
  min?: number;
  /** Maximum value for the adjustment. Defaults to Infinity. */
  max?: number;
  /** Step size for the adjustment. Defaults to 1. */
  step?: number;
  /** Callback when the value changes */
  onValueChange: (value: T) => void;
}

export function DialogAdjustments({
  children,
  preview,
  adjustmentType,
  mapper,
  onCancel: userOnCancel,
  onApply: userOnApply,
}: {
  children: React.ReactNode;
  preview?: Record<string, NumericInputValue>;
  adjustmentType?: DialogFeatureType;
  mapper?: () => Record<string, unknown>;
  onCancel?: (e: React.MouseEvent<HTMLButtonElement>) => void;
  onApply?: (e: React.MouseEvent<HTMLButtonElement>) => void;
}) {
  const mapped = useMemo(() => {
    return typeof mapper === 'function'
      ? mapper()
      : (Object.fromEntries(
          Object.entries(preview || {}).map(([key, val]) => [key, (val as NumericInputValue).num]),
        ) as Record<string, number>);
  }, [preview, mapper]);

  const onApply = async (e: React.MouseEvent<HTMLButtonElement>) => {
    userOnApply?.(e);
    await window.alakazam.adjustments.applyAdjustment('brightness-contrast');
    window.close();
  };

  const onCancel = async (e: React.MouseEvent<HTMLButtonElement>) => {
    userOnCancel?.(e);
    await window.alakazam.adjustments.cancelAdjustment('brightness-contrast');
    window.close();
  };

  const prevMappedRef = useRef<Record<string, unknown> | null>(null);
  const previewTimer = useRef<number | null>(null);

  const shallowEqual = (a?: Record<string, unknown> | null, b?: Record<string, unknown> | null) => {
    if (a === b) return true;
    if (!a || !b) return false;
    const aKeys = Object.keys(a);
    const bKeys = Object.keys(b);
    if (aKeys.length !== bKeys.length) return false;
    for (let i = 0; i < aKeys.length; i++) {
      const k = aKeys[i];
      if (Number((a as Record<string, unknown>)[k]) !== Number((b as Record<string, unknown>)[k])) return false;
    }
    return true;
  };

  useEffect(() => {
    if (!adjustmentType) return;
    if (shallowEqual(prevMappedRef.current, mapped)) return;
    prevMappedRef.current = mapped ?? null;
    if (previewTimer.current) window.clearTimeout(previewTimer.current);
    previewTimer.current = window.setTimeout(() => {
      window.alakazam.adjustments.previewAdjustment(adjustmentType, mapped);
      previewTimer.current = null;
    }, 40);
    return () => {
      if (previewTimer.current) window.clearTimeout(previewTimer.current);
      previewTimer.current = null;
    };
  }, [mapped, adjustmentType]);

  useToggleAdjustments(adjustmentType ?? '', mapped);

  return (
    <div className="flex w-[400px] flex-col gap-10 p-4 **:data-[name=card-content]:space-y-10">
      <div className="flex flex-col gap-10" data-type="adjustment-rows">
        {children}
      </div>
      <div className="flex justify-end gap-2">
        <Button onClick={onCancel} variant="secondary">
          Cancel
        </Button>
        <Button onClick={onApply} variant="primary">
          Apply
        </Button>
      </div>
    </div>
  );
}

export function AdjustmentRow<T = number>({
  label,
  step = 1,
  value,
  options,
  min = -Infinity,
  max = Infinity,
  onValueChange: userOnValueChange,
}: AdjustmentRowProps<T>) {
  const [inputValue, setInputValue] = useState<T | string | undefined>(value);
  const isSettingFromPropRef = useRef(false);
  const [inputDebounce] = useDebounce(inputValue, 250);
  const { onKeyDown, onPaste } = useNumericInputValidation();

  useEffect(() => {
    isSettingFromPropRef.current = true;
    setInputValue(value);
  }, [value]);

  useEffect(() => {
    if (typeof inputDebounce === 'undefined') return;
    // If we just set the inputValue from the prop, don't re-emit it as a change
    // — this differentiates programmatic updates from user-driven changes.
    if (isSettingFromPropRef.current) {
      isSettingFromPropRef.current = false;
      return;
    }
    if ((value as unknown) === (inputDebounce as unknown)) return;
    userOnValueChange(inputDebounce as T);
  }, [inputDebounce, userOnValueChange, value]);

  const isDiscrete = options && options.length > 0;

  const input = useMemo(() => {
    return isDiscrete ? null : (
      <Input
        selectFocus
        scrollWheelIncrement
        value={inputValue as string | number}
        className="w-20"
        onKeyDown={onKeyDown}
        onChange={v => setInputValue(v as unknown as T)}
        onPaste={onPaste}
      />
    );
  }, [inputValue, isDiscrete, onKeyDown, onPaste]);

  const select = useMemo(() => {
    // Replace first letter of each word with uppercase for display
    const titleCase = (str: string) =>
      str
        .toLowerCase()
        .split(/[-_ ]/)
        .map(word => word.charAt(0).toUpperCase() + word.slice(1))
        .join(' ');

    return (
      <Select value={inputValue as string} className="w-60" onSelect={e => setInputValue(e as T)}>
        {options?.map(option => (
          <Option key={option as string} value={option as string}>
            {titleCase(option as string)}
          </Option>
        ))}
      </Select>
    );
  }, [options]);

  return (
    <div data-type="adjustment-row" className="space-y-4">
      <div className="flex items-center justify-between gap-2">
        <label className="block">{label}</label>
        {!isDiscrete && input}
        {isDiscrete && select}
      </div>
      {!isDiscrete && (
        <Slider
          snapAtStart
          step={step}
          min={min}
          max={max}
          value={value as number}
          onDrag={e => setInputValue(e as T)}
        />
      )}
    </div>
  );
}
