import { AdjustmentRow, DialogAdjustments } from '@/client/dialogs/helper/common';
import { useNumericInput } from '@/client/lib/util';

export function BoxBlurDialog() {
  const [radius, setRadius] = useNumericInput(5, { min: 0, max: 100 });
  const ADJUSTMENT_TYPE: DialogFeatureType = 'box-blur';

  return (
    <DialogAdjustments adjustmentType={ADJUSTMENT_TYPE} preview={{ radius }}>
      <AdjustmentRow label="Radius" value={radius.num} min={radius.min} max={radius.max} onValueChange={setRadius} />
    </DialogAdjustments>
  );
}
