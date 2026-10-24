import { runAiModelGuidedAsync, type AbraImageLike } from '@alakazam/mobile';
import { areaMask } from '@/src/lib/native/area-mask';
import { owning } from '@/src/lib/native/native-handles';
import { aiProgress } from '@/src/state/ai-progress';
import { type AiModelRegistration, registerAiModel, runWithAiModel } from '@/src/lib/ai/model-warmth';

/**
 * The capability of a model that fills a masked part of a photo. A model says it has it with `kind: [inpaint]` in its catalog entry
 * (`catalog.yml`), and the tool offers every such model, so adding one changes no code: the manifest says how the model is fed (see
 * `abra-ai-core`, `graph`).
 */
export const INPAINT = 'inpaint';

/** Starts loading before the first removal. Safe to call repeatedly for the same owner and model. */
export const prepareObjectRemoval = (registration: AiModelRegistration): Promise<void> => registerAiModel(registration);

/**
 * Removes the part of `image` that `area` covers and fills it in with what belongs there, using the model of `role`. `area` is rings of
 * `[x, y]` fractions of the photo (see `areaMask`). The model runs on a worker thread, so the app keeps drawing while it works. Changes
 * `image` in place.
 */
export const removeObjects = (image: AbraImageLike, area: number[][][], registration: AiModelRegistration): Promise<void> =>
  owning(async keep => {
    // The bar under the photo moves while the model runs; the model does not say how far along it is.
    aiProgress.next({ ...aiProgress.value, removal: null });
    let failed = false;
    try {
      const mask = areaMask(area, image.width(), image.height(), keep);
      const answer = keep(
        await runWithAiModel(registration, remover =>
          runAiModelGuidedAsync(
            remover,
            image as Parameters<typeof runAiModelGuidedAsync>[1],
            mask as unknown as Parameters<typeof runAiModelGuidedAsync>[2],
          ),
        ),
      );
      image.replaceWith(keep(answer.image()));
    } catch (error) {
      failed = true;
      throw error;
    } finally {
      // A removal that failed did no work to count, so it leaves nothing on the bar.
      const { removal: _earlier, ...others } = aiProgress.value;
      aiProgress.next(failed ? others : { ...aiProgress.value, removal: 1 });
    }
  });
