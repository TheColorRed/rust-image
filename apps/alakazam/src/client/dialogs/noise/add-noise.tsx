import { AdjustmentRow, DialogAdjustments } from '@/client/dialogs/helper/common';
import { useNumericInput } from '@/client/lib/util';
import { useCallback, useState } from 'react';

export function AddNoiseDialog() {
  const [amount, setAmount] = useNumericInput(5, { min: 0, max: 100 });
  const [distribution, setDistribution] = useState<'uniform' | 'gaussian'>('uniform');
  const ADJUSTMENT_TYPE: DialogFeatureType = 'add-noise';

  const mapped = useCallback(() => {
    return { amount: amount.num, distribution };
  }, [amount, distribution]);

  return (
    <DialogAdjustments adjustmentType={ADJUSTMENT_TYPE} mapper={mapped}>
      <AdjustmentRow label="Amount" value={amount.num} min={amount.min} max={amount.max} onValueChange={setAmount} />
      <AdjustmentRow
        label="Distribution"
        value={distribution}
        options={['uniform', 'gaussian']}
        onValueChange={setDistribution}
      />
    </DialogAdjustments>
  );
}
