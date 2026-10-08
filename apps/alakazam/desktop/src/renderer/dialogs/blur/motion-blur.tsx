import { AdjustmentRow, DialogAdjustments } from '@/dialogs/helper/common';
import { useNumericInput } from '@/lib/util';

export function MotionBlurDialog() {
  const [angle, setAngle] = useNumericInput(30, { min: 0, max: 360 });
  const [distance, setDistance] = useNumericInput(15, { min: 1, max: 255 });
  const ADJUSTMENT_TYPE: DialogFeatureType = 'motion-blur';

  return (
    <DialogAdjustments adjustmentType={ADJUSTMENT_TYPE} preview={{ angle, distance }}>
      <AdjustmentRow label="Angle" value={angle.num} min={angle.min} max={angle.max} onValueChange={setAngle} />
      <AdjustmentRow
        label="Distance"
        value={distance.num}
        min={distance.min}
        max={distance.max}
        onValueChange={setDistance}
      />
    </DialogAdjustments>
  );
}
