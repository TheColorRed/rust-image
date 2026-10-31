import { type ImageLike } from '@abra/core/image';
import { type MaskLike } from '@abra/core/mask';
import { app, release } from '@alakazam/sdk';

const ROLE = 'skin-segmentation';
const manifest = (name: string, classes: number[]) => `name: selfie_multiclass_256x256_${name}
description: The ${name} of the six-class selfie segmentation
decoder: semantic-mask
params:
  input_size: 256
  input_layout: nhwc
  input_fit: stretch
  output_layout: nhwc
  class_count: 6
  mask_classes: [${classes.join(', ')}]
classes: [background, hair, body skin, face skin, clothes, others]
`;

/** Makes the hair and skin masks once per project image; each returned mask is owned by the plugin control preload. */
export async function hairMasks(photo: ImageLike): Promise<Record<string, MaskLike> | undefined> {
  if (app.ai.missing([ROLE]).length) return undefined;
  const image = photo.clone();
  const masks: Record<string, MaskLike> = {};
  try {
    for (const [key, name, classes] of [
      ['hair|all', 'hair', [1]],
      ['bodyskin|all', 'skin', [2, 3]],
    ] as const) {
      const output = await app.ai.withManifest(ROLE, manifest(name, [...classes]), model => app.ai.run(model, image));
      try {
        // mask() returns a separately owned copy; the model output handle is no longer needed after it is extracted.
        const mask = output.mask();
        if (mask.bounds(0)) masks[key] = mask;
        else release(mask);
      } finally {
        release(output);
      }
    }
    return masks;
  } catch (error) {
    Object.values(masks).forEach(mask => release(mask));
    throw error;
  } finally {
    release(image);
  }
}

