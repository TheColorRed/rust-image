import { gaussianBlur } from '@abra/core/filters';
import { Mask, type MaskLike } from '@abra/core/mask';
import { owning, release, type Preloaded } from '@alakazam/sdk';
import { makeupMasks } from '../makeup/makeup-models';

export { makeupMasks };
const PARTS = ['lipstick|', 'mouth|'];
const MOUTH_SHARE = 0.5;
const MOUTH_FADE = 0.12;

/** Softens skin edits across lips and mouth while leaving the face-analysis masks intact. */
export const skinWithMouthEased = (
  skin: MaskLike,
  preloaded: Preloaded | undefined,
  keep: <T>(value: T) => T,
  ease = MOUTH_SHARE,
  parts = PARTS,
): MaskLike => {
  if (!preloaded || Array.isArray(preloaded)) return skin;
  let eased = skin;
  const share = keep(Mask.fromValues(skin.width(), skin.height(), new Uint8Array(skin.width() * skin.height()).fill(Math.round(255 * ease)).buffer));
  for (const [key, part] of Object.entries(preloaded)) {
    if (!parts.some(start => key.startsWith(start))) continue;
    const bounds = (part as MaskLike).bounds(0);
    if (!bounds) continue;
    const blurred = keep((part as MaskLike).toImage());
    keep(gaussianBlur(bounds.width * MOUTH_FADE)).apply(blurred);
    eased = keep(eased.subtracted(keep(keep(Mask.fromImage(blurred)).multiplied(share))));
  }
  return eased;
};


