import { type ImageLike } from '@abra/core/image';
import { type MaskLike } from '@abra/core/mask';
import { app, release } from '@alakazam/sdk';

const ROLE = 'skin-segmentation';
const CLASSES = ['background', 'hair', 'body skin', 'face skin', 'clothes', 'others'];

/** Makes the hair and skin masks once per project image; each returned mask is owned by the plugin control preload. */
export async function hairMasks(photo: ImageLike): Promise<Record<string, MaskLike> | undefined> {
  if (app.ai.missing([ROLE]).length) return undefined;
  const image = photo.clone();
  const masks: Record<string, MaskLike> = {};
  try {
    for (const [key, name, maskClasses] of [
      ['hair|all', 'hair', [1]],
      ['bodyskin|all', 'skin', [2, 3]],
    ] as const) {
      const mask = await app.ai.segment(ROLE, image, { name: `selfie_multiclass_256x256_${name}`, inputSize: 256, classes: CLASSES, maskClasses: [...maskClasses] });
      if (mask) masks[key] = mask;
    }
    return masks;
  } catch (error) {
    Object.values(masks).forEach(mask => release(mask));
    throw error;
  } finally {
    release(image);
  }
}
