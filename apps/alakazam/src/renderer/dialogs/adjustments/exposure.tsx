import { AdjustmentRow, DialogAdjustments } from '@/dialogs/helper/common';
import { useNumericInput } from '@/lib/util';
import { Option } from '@/ui/option';
import { Select } from '@/ui/select';
import { useCallback, useState } from 'react';

export function ExposureDialog() {
  const [exposure, setExposure] = useNumericInput(0, { min: -20, max: 20, round: false, step: 0.01 });
  const [offset, setOffset] = useNumericInput(0, { min: -0.5, max: 0.5, round: false, step: 0.01 });
  const [gamma, setGamma] = useNumericInput(1, { min: 0.01, max: 9.99, round: false, step: 0.01 });
  const [preset, setPreset] = useState<'default' | '-1' | '-2' | '+1' | '+2' | 'custom'>('default');
  const ADJUSTMENT_TYPE: DialogFeatureType = 'exposure';

  const setAll = useCallback(
    (exposureVal: number, offsetVal: number, gammaVal: number) => {
      setExposure(exposureVal);
      setOffset(offsetVal);
      setGamma(gammaVal);
    },
    [setExposure, setOffset, setGamma],
  );

  const onPresetChange = useCallback(
    (value: typeof preset) => {
      // Set the preset first to update selection UI, then apply values to the
      // numeric inputs. This reduces the chance that inputs will re-emit their
      // initial value during the transition.
      setPreset(value);
      if (value === 'default') setAll(0, 0, 1);
      else if (value === '-1') setAll(-1, 0, 1);
      else if (value === '-2') setAll(-2, 0, 1);
      else if (value === '+1') setAll(1, 0, 1);
      else if (value === '+2') setAll(2, 0, 1);
      // log after setting
    },
    [setAll],
  );

  const onChangeAny = useCallback(() => {
    // If any value is changed manually, switch to 'custom' preset
    setPreset('custom');
  }, []);

  return (
    <DialogAdjustments adjustmentType={ADJUSTMENT_TYPE} preview={{ exposure, offset, gamma }}>
      <Select onSelect={onPresetChange} value={preset}>
        <Option value="default">Default</Option>
        <Option value="-1">Minus 1</Option>
        <Option value="-2">Minus 2</Option>
        <Option value="+1">Plus 1</Option>
        <Option value="+2">Plus 2</Option>
        <Option value="custom">Custom</Option>
      </Select>
      <AdjustmentRow
        label="Exposure"
        value={exposure.num}
        min={exposure.min}
        max={exposure.max}
        step={exposure.step}
        onValueChange={v => [setExposure(v), onChangeAny()]}
      />
      <AdjustmentRow
        label="Offset"
        value={offset.num}
        min={offset.min}
        max={offset.max}
        step={offset.step}
        onValueChange={v => [setOffset(v), onChangeAny()]}
      />
      <AdjustmentRow
        label="Gamma"
        value={gamma.num}
        min={gamma.min}
        max={gamma.max}
        step={gamma.step}
        onValueChange={v => [setGamma(v), onChangeAny()]}
      />
    </DialogAdjustments>
  );
}
