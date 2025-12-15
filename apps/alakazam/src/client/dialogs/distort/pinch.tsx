import { AdjustmentRow, DialogAdjustments } from '@/client/dialogs/helper/common';
import { useNumericInput } from '@/client/lib/util';

export function PinchDialog() {
  const [amount, setAmount] = useNumericInput(0, { min: -100, max: 100 });
  const ADJUSTMENT_TYPE: DialogFeatureType = 'pinch';

  return (
    <DialogAdjustments adjustmentType={ADJUSTMENT_TYPE} preview={{ amount }}>
      <AdjustmentRow label="Amount" value={amount.num} min={amount.min} max={amount.max} onValueChange={setAmount} />
    </DialogAdjustments>
  );
}
