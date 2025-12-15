declare type DialogFeature = 'blur' | 'distort' | 'noise' | 'adjustments';

declare type DialogFeatureType =
  // Adjustments
  | 'brightness-contrast'
  | 'exposure'
  | 'vibrance'
  // Blurs
  | 'box-blur'
  | 'gaussian-blur'
  | 'lens-blur'
  | 'motion-blur'
  | 'surface-blur'
  // Distort
  | 'pinch'
  | 'ripple'
  // Noise
  | 'add-noise'
  | 'despeckle'
  | 'median';

declare type DialogPath = `${DialogFeature}/${DialogFeatureType}`;

declare interface DialogSettings {
  properties?: { multiple: boolean };
  filters?: { name: string; extensions: string[] }[];
}
