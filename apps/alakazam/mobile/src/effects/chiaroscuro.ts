import { contrast, exposure, grayscale, highlightsShadows as toneAdjustment } from '@abra/core/adjustments';
import { blend, BlendMode } from '@abra/core/blend';
import { gaussianBlur } from '@abra/core/filters';
import { type AbraImageLike } from '@alakazam/mobile';
import { owning } from '@/src/lib/native/native-handles';

/** Neighborhood radius for the shadow recovery, as a fraction of photo width. Wider than the shared default so areas blend smoothly. */
const SHADOW_RADIUS = 0.05;
/** Width of the tonal transition as a fraction. Wider keeps the edge between light and shadow soft. */
const TONE_WIDTH = 0.8;
/** Blur of the photo's color before it is recombined with the new brightness, as a fraction of photo width. Hides the block pattern from compressed color. */
const COLOR_SMOOTH = 0.01;

/**
 * Deepens the shadows of `photo` in place, keeping its own colors, like a single light falling across a subject. At 0 the photo
 * is unchanged. Higher values push the shadows toward black and make the lights brighter. The darkening is done on a copy and
 * only its brightness is taken, so skin and other colors keep their hue instead of turning redder. `color` is how much of the
 * photo's color is kept: 1 is all of it, 0 is a black and white photo.
 */
export const chiaroscuro = (photo: AbraImageLike, amount: number, color = 1): void =>
  owning((keep) => {
    if (color < 1) {
      const gray = keep(photo.clone());
      keep(grayscale()).apply(gray);
      keep(keep(blend(gray)).withOpacity(1 - color)).apply(photo);
    }
    if (amount <= 0) return;
    const toned = keep(photo.clone());
    keep(contrast(amount * 60)).apply(toned);
    keep(exposure(-amount * 0.5).withGamma(1 + amount * 0.5)).apply(toned);
    const tones = keep(toneAdjustment(amount * 20, -amount * 100).withTone(TONE_WIDTH));
    keep(tones.withRadius(toned.width() * SHADOW_RADIUS)).apply(toned);
    const smoothColor = keep(photo.clone());
    keep(gaussianBlur(Math.max(2, toned.width() * COLOR_SMOOTH))).apply(smoothColor);
    keep(keep(blend(toned)).withMode(BlendMode.Luminosity)).apply(smoothColor);
    keep(blend(smoothColor)).apply(photo);
  });
