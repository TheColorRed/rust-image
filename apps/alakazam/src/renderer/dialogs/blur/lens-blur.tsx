import { AdjustmentRow, DialogAdjustments } from '@/dialogs/helper/common';
import { useNumericInput } from '@/lib/util';
import { Card, CardContent } from '@/ui/card';
import { useCallback, useState } from 'react';

type IrisShape = 'triangle' | 'square' | 'pentagon' | 'hexagon' | 'heptagon' | 'octagon';
type DistributionType = 'uniform' | 'gaussian';

export function LensBlurDialog() {
  const [irisRadius, setIrisRadius] = useNumericInput(15, { min: 0, max: 100 });
  const [irisShape, setIrisShape] = useState<IrisShape>('triangle');
  const [irisBladeCurvature, setIrisBladeCurvature] = useNumericInput(0, { min: 0, max: 100 });
  const [irisRotation, setIrisRotation] = useNumericInput(0, { min: 0, max: 360 });
  const [specularBrightness, setSpecularBrightness] = useNumericInput(0, { min: 0, max: 100 });
  const [specularThreshold, setSpecularThreshold] = useNumericInput(255, { min: 0, max: 255 });
  const [noiseAmount, setNoiseAmount] = useNumericInput(0, { min: 0, max: 100 });
  const [noiseDistribution, setNoiseDistribution] = useState<DistributionType>('uniform');
  const [samples, setSamples] = useNumericInput(16, { min: 1, max: 64 });
  const ADJUSTMENT_TYPE: DialogFeatureType = 'lens-blur';

  const mapper = useCallback(() => {
    return {
      iris: {
        shape: irisShape,
        radius: irisRadius.num,
        bladeCurvature: irisBladeCurvature.num,
        rotation: irisRotation.num,
      },
      specular: {
        brightness: specularBrightness.num,
        threshold: specularThreshold.num,
      },
      noise: {
        amount: noiseAmount.num,
        distribution: noiseDistribution,
      },
      samples: samples.num,
    };
  }, [
    irisShape,
    irisRadius,
    irisBladeCurvature,
    irisRotation,
    specularBrightness,
    specularThreshold,
    noiseAmount,
    noiseDistribution,
    samples,
  ]);

  return (
    <DialogAdjustments adjustmentType={ADJUSTMENT_TYPE} mapper={mapper}>
      <Card variant="bordered" label="Iris">
        <CardContent>
          <AdjustmentRow
            label="Radius"
            value={irisRadius.num}
            min={irisRadius.min}
            max={irisRadius.max}
            onValueChange={setIrisRadius}
          />
          <AdjustmentRow
            label="Blade Curvature"
            value={irisBladeCurvature.num}
            min={irisBladeCurvature.min}
            max={irisBladeCurvature.max}
            onValueChange={setIrisBladeCurvature}
          />
          <AdjustmentRow
            label="Rotation"
            value={irisRotation.num}
            min={irisRotation.min}
            max={irisRotation.max}
            onValueChange={setIrisRotation}
          />
        </CardContent>
      </Card>
      <Card variant="bordered" label="Specular Highlights">
        <CardContent>
          <AdjustmentRow
            label="Brightness"
            value={specularBrightness.num}
            min={specularBrightness.min}
            max={specularBrightness.max}
            onValueChange={setSpecularBrightness}
          />
          <AdjustmentRow
            label="Threshold"
            value={specularThreshold.num}
            min={specularThreshold.min}
            max={specularThreshold.max}
            onValueChange={setSpecularThreshold}
          />
        </CardContent>
      </Card>
      <Card variant="bordered" label="Noise">
        <CardContent>
          <AdjustmentRow
            label="Amount"
            value={noiseAmount.num}
            min={noiseAmount.min}
            max={noiseAmount.max}
            onValueChange={setNoiseAmount}
          />
          <AdjustmentRow
            label="Distribution"
            options={['uniform', 'gaussian']}
            value={noiseDistribution}
            onValueChange={setNoiseDistribution}
          />
        </CardContent>
      </Card>
    </DialogAdjustments>
  );
}
