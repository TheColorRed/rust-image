import { NIGHT_VISION_DEFAULTS, nightVision } from '@/src/effects/night-vision';
import { SliderControl } from '@/src/lib/project/edit-sections';

export const nightVisionControl = {
  kind: 'slider',
  key: 'action-night-vision',
  label: 'Night Vision',
  sliderLabel: ['Strength', 'Grain', 'Brightness', 'Midtones', 'Vignette', 'Barrel', 'Scanlines'],
  display: 'tabulated',
  pillFormat: ['percent', 'percent', 'plain', 'plain', 'percent', 'signed', 'percent'],
  min: [0, 0, 0.5, 1, 0, -0.5, 0],
  max: [1, 0.3, 2, 2, 1, 0.5, 1],
  step: [0.01, 0.01, 0.05, 0.05, 0.01, 0.01, 0.01],
  defaultValue: [
    NIGHT_VISION_DEFAULTS.strength,
    NIGHT_VISION_DEFAULTS.grain,
    NIGHT_VISION_DEFAULTS.gain,
    NIGHT_VISION_DEFAULTS.power,
    NIGHT_VISION_DEFAULTS.vignette,
    NIGHT_VISION_DEFAULTS.barrel,
    NIGHT_VISION_DEFAULTS.scanlines,
  ],
  applyOnSelect: true,
  tool: (photo, [strength, grain, gain, power, vignette, barrel, scanlines]: number[]) =>
    nightVision(photo, { strength, grain, gain, power, vignette, barrel, scanlines }),
  thumb: photo => nightVision(photo, NIGHT_VISION_DEFAULTS),
} satisfies SliderControl<number>;
