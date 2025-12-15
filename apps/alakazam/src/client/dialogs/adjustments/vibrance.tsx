import { AdjustmentRow, DialogAdjustments } from '@/client/dialogs/helper/common';
import { useNumericInput } from '@/client/lib/util';

export function VibranceDialog() {
  const [vibrance, setVibrance] = useNumericInput(0, { min: -100, max: 100 });
  const [saturation, setSaturation] = useNumericInput(0, { min: -100, max: 100 });
  const ADJUSTMENT_TYPE: DialogFeatureType = 'vibrance';

  return (
    <DialogAdjustments adjustmentType={ADJUSTMENT_TYPE} preview={{ vibrance, saturation }}>
      <AdjustmentRow
        label="Vibrance"
        value={vibrance.num}
        min={vibrance.min}
        max={vibrance.max}
        step={vibrance.step}
        onValueChange={setVibrance}
      />
      <AdjustmentRow
        label="Saturation"
        value={saturation.num}
        min={saturation.min}
        max={saturation.max}
        step={saturation.step}
        onValueChange={setSaturation}
      />
    </DialogAdjustments>
  );
}
