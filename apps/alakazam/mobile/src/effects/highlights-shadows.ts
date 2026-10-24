import { type HighlightsShadows, highlightsShadows as toneAdjustment } from '@abra/core/adjustments';
import { ImageLike } from '@abra/core/image';
import { release } from '@/src/lib/native/native-handles';

/** Neighborhood radius as a fraction of photo width, shared by preview and export. */
const RADIUS = 0.04;
/** Tonal width as a fraction (0.5 = 50%). The basic screen keeps these advanced settings internal. */
const TONE = 0.5;

/** Local luminance adjustment with smooth tonal transitions and color preservation. */
export const highlightsShadows = (photo: ImageLike, highlights: number, shadows: number) => {
  if (highlights === 0 && shadows === 0) return;
  const base = toneAdjustment(highlights, shadows);
  const tones = base.withTone(TONE);
  const effect = tones.withRadius(photo.width() * RADIUS);
  try {
    effect.apply(photo);
  } finally {
    release(effect);
    release(tones);
    release(base);
  }
};
