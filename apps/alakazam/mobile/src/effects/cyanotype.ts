import { autoTone, gradientMap } from '@abra/core/adjustments';
import { blend } from '@abra/core/blend';
import { Color } from '@abra/core/color';
import { ColorStop, Gradient } from '@abra/core/gradient';
import { type AbraImageLike } from '@alakazam/mobile';
import { owning } from '@/src/lib/native/native-handles';

/** The deepest tone, Prussian blue, that the shadows are mapped to. */
const PRUSSIAN_BLUE: [number, number, number] = [0, 49, 83];
/** The highlight tone that the lights are mapped to. */
const HIGHLIGHT: [number, number, number] = [255, 255, 255];

/**
 * Turns `photo` into a cyanotype print in place: its brightness is mapped to a two tone gradient from Prussian blue in the
 * shadows to white in the highlights, which drops every other color. At 0 the photo is unchanged and at 1 it is fully cyanotype.
 * Values in between fade from one to the other.
 */
export const cyanotype = (photo: AbraImageLike, amount: number): void =>
  owning(keep => {
    if (amount <= 0) return;
    const blue = keep(Color.fromRgb(...PRUSSIAN_BLUE));
    const white = keep(Color.fromRgb(...HIGHLIGHT));
    const gradient = keep(new Gradient([keep(new ColorStop(blue, 0)), keep(new ColorStop(white, 1))]));
    const print = keep(photo.clone());
    keep(autoTone()).apply(print);
    keep(gradientMap(gradient)).apply(print);
    keep(keep(blend(print)).withOpacity(amount)).apply(photo);
  });
