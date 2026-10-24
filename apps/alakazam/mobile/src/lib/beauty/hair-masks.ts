import { devDebug } from '@/src/lib/log';
import { AiModel } from '@abra/core';
import { type ImageLike } from '@abra/core/image';
import { type MaskLike } from '@abra/core/mask';
import { aiModelPath, runAiModelAsync } from '@alakazam/mobile';
import { aiProgress } from '@/src/state/ai-progress';
import { EDGE_REFINEMENT, HAIR_MODELS, SKIN_SEGMENTATION, model, unloadModel } from '@/src/lib/ai/faces';
import { edgeMatte } from '@/src/lib/ai/edge-matte';
import { owning, release } from '@/src/lib/native/native-handles';

/**
 * The skin model read for its hair class instead of its skin classes. It is the same `.onnx` file, so it needs no download of its own:
 * only the manifest differs (`mask_classes` is the hair, class 1). The model looks at the whole photo, so it finds hair far from the face.
 */
const selfieManifest = (name: string, classes: number[]) => `name: selfie_multiclass_256x256_${name}
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

/**
 * What is found in the whole photo with the model: the hair (class 1), and the skin of the face and body (classes 2 and 3), so the
 * hair can be kept off the skin. Each is the same file read with a different manifest, so the hair is `hair|all` and the skin `bodyskin|all`.
 */
const WHOLE_PHOTO = [
  { key: 'hair|all', manifest: selfieManifest('hair', [1]) },
  { key: 'bodyskin|all', manifest: selfieManifest('skin', [2, 3]) },
];


/**
 * Runs the hair analysis once and keeps what Hair Color is made from: the hair of the whole photo (`hair|all`), the skin of the face and
 * body to keep it off (`bodyskin|all`), and, with the edge refinement model on this device, the hair's alpha (`hair|matte`). It is
 * the hair controls' own analysis, so the other Beauty controls never pay for the edge model. The models run in the background, like
 * `makeupMasks`; the bar under the photo counts it as one task, moving while the two models run and then as far as the edge tiles have got.
 */
export const hairMasks = async (photo: ImageLike): Promise<Record<string, MaskLike> | undefined> => {
  if (HAIR_MODELS.some(role => !model(role))) return undefined;
  const mine = photo.clone();
  const made: Record<string, MaskLike> = {};
  aiProgress.next({ ...aiProgress.value, hair: null });
  try {
    await owning(async keep => {
      // The hair and the skin are found in the whole photo, once each.
      for (const { key, manifest } of WHOLE_PHOTO) {
        // Loaded for this run only and freed after it: each manifest is the same file, and holding both keeps the model's memory twice.
        const segmenter = AiModel.loadWithManifest(aiModelPath(SKIN_SEGMENTATION), manifest);
        let found: Awaited<ReturnType<typeof runAiModelAsync>>;
        try {
          found = keep(await runAiModelAsync(segmenter, mine as Parameters<typeof runAiModelAsync>[1]));
        } finally {
          release(segmenter);
        }
        // Kept in `made`, so it is not freed with the rest; one with nothing in it is.
        const segment = found.mask();
        if (segment.bounds(0)) made[key] = segment;
        else release(segment);
      }
      // With the edge refinement model on this device, it decides the pixels along the hair's edge (see `applyHairColor`).
      const refiner = model(EDGE_REFINEMENT);
      if (refiner && made['hair|all']) {
        const started = Date.now();
        let matte: MaskLike | undefined;
        try {
          matte = await edgeMatte(refiner, mine, made['hair|all'], keep, share => aiProgress.next({ ...aiProgress.value, hair: share }));
        } finally {
          // The model holds gigabytes of memory after it has run, and it runs once per photo, so it is not kept.
          unloadModel(EDGE_REFINEMENT);
        }
        if (matte) made['hair|matte'] = matte;
        devDebug('hair', 'edge refinement ms:', Date.now() - started, 'made:', !!matte);
      } else {
        devDebug('hair', 'edge refinement skipped:', refiner ? 'no hair found' : 'model not downloaded');
      }
    });
    aiProgress.next({ ...aiProgress.value, hair: 1 });
  } catch (error) {
    // A task that fails takes itself off the bar, because it did no work to count.
    const { hair: _failed, ...others } = aiProgress.value;
    aiProgress.next(others);
    Object.values(made).forEach(mask => release(mask));
    throw error;
  } finally {
    release(mine);
  }
  return made;
};

