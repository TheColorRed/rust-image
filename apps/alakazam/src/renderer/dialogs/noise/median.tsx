import { AdjustmentRow, DialogAdjustments } from '@/dialogs/helper/common';
import { useNumericInput } from '@/lib/util';

export function MedianDialog() {
  const [radius, setRadius] = useNumericInput(5, { min: 0, max: 500 });
  const ADJUSTMENT_TYPE: DialogFeatureType = 'median';

  return (
    <DialogAdjustments adjustmentType={ADJUSTMENT_TYPE} preview={{ radius }}>
      <AdjustmentRow label="Radius" value={radius.num} min={radius.min} max={radius.max} onValueChange={setRadius} />
    </DialogAdjustments>
  );
}
