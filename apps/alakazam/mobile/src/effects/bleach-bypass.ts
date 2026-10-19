import { contrast, grayscale, saturation } from '@abra/core/adjustments';
import { blend, BlendMode } from '@abra/core/blend';
import { type AbraImageLike } from '@alakazam/mobile';
import { owning } from '@/src/lib/native/native-handles';

/** How far the colors are muted at full strength, from 0 (unchanged) to -100 (black and white). */
const MUTE = -60;
/** How much contrast is added at full strength. */
const CONTRAST = 45;

/**
 * Turns `photo` into a bleach bypass print in place, as if the bleaching step was skipped so the silver stayed in the film.
 * A grayscale copy is laid over the photo in Overlay mode, which drives up contrast and deepens the shadows, then the colors
 * are muted and contrast is added. At 0 the photo is unchanged and at 1 it is fully bleach bypass. Values in between fade
 * from one to the other.
 */
export const bleachBypass = (photo: AbraImageLike, amount: number): void =>
  owning(keep => {
    if (amount <= 0) return;
    const silver = photo.clone();
    const bypass = photo.clone();
    const base = grayscale();
    const layer = blend(silver);
    const mode = layer.withMode(BlendMode.Overlay);
    const muted = saturation(MUTE * amount);
    const punch = contrast(CONTRAST * amount);
    const result = blend(bypass);
    const opacity = result.withOpacity(amount);
    keep(silver, bypass, base, layer, mode, muted, punch, result, opacity);
    base.apply(silver);
    mode.apply(bypass);
    muted.apply(bypass);
    punch.apply(bypass);
    opacity.apply(photo);
  });
