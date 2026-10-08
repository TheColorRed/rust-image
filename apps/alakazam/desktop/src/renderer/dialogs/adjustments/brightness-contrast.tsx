import { AdjustmentRow, DialogAdjustments } from '@/dialogs/helper/common';
import { useNumericInput } from '@/lib/util';

export function BrightnessContrastDialog() {
  const [brightness, setBrightness] = useNumericInput(0, { min: -100, max: 100 });
  const [contrast, setContrast] = useNumericInput(0, { min: -100, max: 100 });
  const ADJUSTMENT_TYPE: DialogFeatureType = 'brightness-contrast';

  return (
    <DialogAdjustments adjustmentType={ADJUSTMENT_TYPE} preview={{ brightness, contrast }}>
      <AdjustmentRow
        label="Brightness"
        value={brightness.num}
        min={brightness.min}
        max={brightness.max}
        onValueChange={setBrightness}
      />
      <AdjustmentRow
        label="Contrast"
        value={contrast.num}
        min={contrast.min}
        max={contrast.max}
        onValueChange={setContrast}
      />
    </DialogAdjustments>
  );
}
