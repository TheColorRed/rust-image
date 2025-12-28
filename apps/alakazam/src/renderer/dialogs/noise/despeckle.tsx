import { AdjustmentRow, DialogAdjustments } from '@/dialogs/helper/common';
import { useNumericInput } from '@/lib/util';

export function DespeckleDialog() {
  const [radius, setRadius] = useNumericInput(5, { min: 0, max: 30 });
  const [threshold, setThreshold] = useNumericInput(0, { min: 0, max: 255 });
  const ADJUSTMENT_TYPE: DialogFeatureType = 'despeckle';

  return (
    <DialogAdjustments adjustmentType={ADJUSTMENT_TYPE} preview={{ radius, threshold }}>
      <AdjustmentRow label="Radius" value={radius.num} min={radius.min} max={radius.max} onValueChange={setRadius} />
      <AdjustmentRow
        label="Threshold"
        value={threshold.num}
        min={threshold.min}
        max={threshold.max}
        onValueChange={setThreshold}
      />
    </DialogAdjustments>
  );
}
