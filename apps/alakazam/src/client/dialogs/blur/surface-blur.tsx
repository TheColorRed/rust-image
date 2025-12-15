import { AdjustmentRow, DialogAdjustments } from '@/client/dialogs/helper/common';
import { useNumericInput } from '@/client/lib/util';

export function SurfaceBlurDialog() {
  const [radius, setRadius] = useNumericInput(5, { min: 1, max: 100 });
  const [threshold, setThreshold] = useNumericInput(15, { min: 2, max: 255 });
  const ADJUSTMENT_TYPE: DialogFeatureType = 'surface-blur';

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
