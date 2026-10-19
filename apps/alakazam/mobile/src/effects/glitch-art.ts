import { blend } from '@abra/core/blend';
import { type AbraImageLike } from '@alakazam/mobile';
import { owning } from '@/src/lib/native/native-handles';

/**
 * Turns `photo` into a glitch-art look by sliding a copy of it and laying only its green and blue channels over the photo. The
 * red channel stays in place, so the shifted edges show as a neon fringe. `offsetX` and `offsetY` move the copy by that many
 * pixels, and `strength` fades the fringe. Zero offsets leave the photo unchanged.
 */
export const glitchArt = (photo: AbraImageLike, strength: number, offsetX: number, offsetY: number): void =>
  owning(keep => {
    const copy = keep(photo.clone());
    const shifted = keep(keep(blend(copy)).withOffset({ x: offsetX, y: offsetY }));
    keep(shifted.withOpacity(strength).withChannels({ red: false, green: true, blue: true })).apply(photo);
  });
