import { AdjustmentRow, DialogAdjustments } from '@/client/dialogs/helper/common';
import { useNumericInput } from '@/client/lib/util';
import { useCallback, useEffect, useState } from 'react';

export function RippleDialog() {
  const [amount, setAmount] = useNumericInput(20, { min: -100, max: 100 });
  const [angle, setAngle] = useNumericInput(360, { min: 0, max: 360 });
  const [size, setSize] = useState<'small' | 'medium' | 'large'>('medium');
  const [shape, setShape] = useState<'circular' | 'square' | 'random' | 'angular'>('angular');
  const ADJUSTMENT_TYPE: DialogFeatureType = 'ripple';

  const mapper = useCallback(() => {
    return { amount: amount.num, size, shape: shape === 'angular' ? angle.num : shape };
  }, [size, shape, amount, angle]);

  useEffect(() => {
    window.alakazam.updateDialogWindowSize();
  }, [angle]);

  return (
    <DialogAdjustments adjustmentType={ADJUSTMENT_TYPE} mapper={mapper}>
      <AdjustmentRow label="Amount" value={amount.num} min={amount.min} max={amount.max} onValueChange={setAmount} />
      {shape === 'angular' && (
        <AdjustmentRow label="Angle" value={angle.num} min={angle.min} max={angle.max} onValueChange={setAngle} />
      )}
      <AdjustmentRow label="Size" value={size} options={['small', 'medium', 'large']} onValueChange={setSize} />
      <AdjustmentRow
        label="Shape"
        value={shape}
        options={['angular', 'circular', 'square', 'random']}
        onValueChange={setShape}
      />
    </DialogAdjustments>
  );
}
