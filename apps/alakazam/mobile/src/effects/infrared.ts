import { channelMix, invert } from '@abra/core/adjustments';
import { blend, BlendMode } from '@abra/core/blend';
import { type AbraImageLike } from '@alakazam/mobile';
import { owning } from '@/src/lib/native/native-handles';

/**
 * Turns `photo` into a false-color infrared look in place. At 0 the colors are inverted and their red and blue swapped, which
 * gives pink foliage and deep blue skies and water. Blending in Color mode keeps the photo's own brightness. At 1 the pink keeps
 * its hue but takes its brightness from a green-heavy mix, so foliage goes pale and skies and water go dark. Values in between
 * fade from one to the other.
 */
export const infrared = (photo: AbraImageLike, amount: number): void =>
  owning(keep => {
    const pink = keep(photo.clone());
    const swapped = keep(photo.clone());
    keep(invert()).apply(swapped);
    keep(channelMix([0, 0, 1], [0, 1, 0], [1, 0, 0])).apply(swapped);
    keep(keep(blend(swapped)).withMode(BlendMode.Color)).apply(pink);

    const light = keep(photo.clone());
    keep(channelMix([0.2, 1.4, -0.6], [0.2, 1.4, -0.6], [0.2, 1.4, -0.6])).apply(light);
    const white = keep(pink.clone());
    keep(keep(blend(light)).withMode(BlendMode.Luminosity)).apply(white);
    keep(keep(blend(white)).withOpacity(amount)).apply(pink);

    keep(blend(pink)).apply(photo);
  });
