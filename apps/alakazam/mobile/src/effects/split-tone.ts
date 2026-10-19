import { autoTone, gradientMap } from '@abra/core/adjustments';
import { blend } from '@abra/core/blend';
import { Color } from '@abra/core/color';
import { ColorStop, Gradient } from '@abra/core/gradient';
import { type AbraImageLike } from '@alakazam/mobile';
import { owning } from '@/src/lib/native/native-handles';

/** The deepest tone, a cool selenium blue, kept to the darkest shadows only. */
const SELENIUM: [number, number, number] = [10, 20, 28];
/** A dark sepia brown, so the shadows stay warm rather than turning blue. */
const DEEP_SEPIA: [number, number, number] = [44, 32, 24];
/** The middle tone, a warm sepia brown that most of the photo sits in. */
const MIDDLE: [number, number, number] = [150, 120, 92];
/** The highlight tone, a light warm cream. */
const SEPIA: [number, number, number] = [246, 234, 212];

/**
 * Turns `photo` into a split toned print in place: its brightness is mapped to a warm sepia gradient, with a cool selenium blue
 * only in the deepest shadows, so the two ends of the tone are different temperatures. At 0 the photo is
 * unchanged and at 1 it is fully split toned. Values in between fade from one to the other.
 */
export const splitTone = (photo: AbraImageLike, amount: number): void =>
  owning(keep => {
    if (amount <= 0) return;
    const shadow = Color.fromRgb(...SELENIUM);
    const deep = Color.fromRgb(...DEEP_SEPIA);
    const middle = Color.fromRgb(...MIDDLE);
    const highlight = Color.fromRgb(...SEPIA);
    const shadowStop = new ColorStop(shadow, 0);
    const deepStop = new ColorStop(deep, 0.2);
    const middleStop = new ColorStop(middle, 0.5);
    const highlightStop = new ColorStop(highlight, 1);
    const gradient = new Gradient([shadowStop, deepStop, middleStop, highlightStop]);
    const print = photo.clone();
    const tone = autoTone();
    const map = gradientMap(gradient);
    const layer = blend(print);
    const opacity = layer.withOpacity(amount);
    keep(shadow, deep, middle, highlight, shadowStop, deepStop, middleStop, highlightStop, gradient, print, tone, map, layer, opacity);
    tone.apply(print);
    map.apply(print);
    opacity.apply(photo);
  });
