import { devDebug } from '@/src/lib/log';
import { watchForStalls } from '@/src/lib/dev/stall-monitor';
import { type ImageLike } from '@abra/core/image';
import { type MaskLike } from '@abra/core/mask';
import {
  ImageWorkState,
  edgeMatteAsync,
  fitBodyPartsAsync,
  maximumMasksAsync,
  personDetectionDownloaded,
} from '@alakazam/mobile';
import { detectPeople } from '@/src/lib/ai/analysis';
import { missingModels } from '@/src/lib/ai/ai-model-availability';
import { EDGE_REFINEMENT, SKIN_SEGMENTATION } from '@/src/lib/ai/faces';
import { aiProgress } from '@/src/state/ai-progress';
import { release } from '@/src/lib/native/native-handles';
import { preloadSession } from '@/src/lib/editor/editor-preloads';

/** How much of the photo around the people the body model is given, as a share of their size: a person who is small in the photo is only a few pixels to a model that sees a small copy. */
const PARTS_MARGIN = 0.1;
/** How far each part follows the edges of the photo, as a share of the people's size. */
const PARTS_FOLLOW = 0.012;

const destroy = (mask: MaskLike) => release(mask);

/**
 * The cutout of the people in a photo, as a mask of the whole photo (`subject`): white on the people, black elsewhere. With the body model on
 * this device it also gives the parts of the people, each followed to the edges of the photo and kept inside the cutout: `hair`, `skin`,
 * `clothes` and `accessories`. The cutout includes the accessories (a hat the person finder left out), so every part is inside it. The person finder gives a
 * rough mask for each person, which are joined, and the edge refinement model, when it is on this device, decides the pixels along their
 * edge, hair included. Without that model the rough masks are the cutout. Nothing when the person finder is not on this device or finds
 * nobody, which a caller answers by not using a cutout. It runs in the background as a `preload`, once for a photo.
 */
export const subjectMask = async (photo: ImageLike): Promise<Record<string, MaskLike> | undefined> => {
  if (!personDetectionDownloaded()) return undefined;
  const mine = photo.clone();
  const session = preloadSession();
  const state = new ImageWorkState();
  const temporary: MaskLike[] = [];
  aiProgress.next({ ...aiProgress.value, subject: null });
  const stopWatching = watchForStalls('cutting out the people');
  // Development: how long each phase held the JavaScript thread or waited, to see which one makes the screen slow.
  let phaseStarted = Date.now();
  const phase = (name: string) => {
    devDebug('subject', name, Date.now() - phaseStarted, 'ms');
    phaseStarted = Date.now();
  };
  try {
    // The people are found once for the photo and shared, so their masks are not freed here.
    const found = await detectPeople(mine as Parameters<typeof detectPeople>[0], { silent: true });
    if (session !== preloadSession()) return undefined;
    phase('found the people');
    const rough = await maximumMasksAsync(
      found.map(person => person.mask),
      mine.width(),
      mine.height(),
    );
    if (session !== preloadSession()) return undefined;
    if (!rough) {
      aiProgress.next({ ...aiProgress.value, subject: 1 });
      return undefined;
    }
    temporary.push(rough);
    phase('joined the rough masks');
    let subject: MaskLike | undefined;
    if (missingModels([EDGE_REFINEMENT]).length === 0) {
      // Asks the worker how far it is, but tells the screen only when that changed, and to a whole percent: every push to `aiProgress` draws
      // the progress bar, the banner and the editor again, and the same number pushed every 80 ms kept the JavaScript thread busy for as long as
      // the refinement ran (a dozen draws of the editor a second).
      let shown = -1;
      const poll = setInterval(() => {
        if (session !== preloadSession()) return state.cancel();
        const share = Math.round(state.share() * 100) / 100;
        if (share === shown) return;
        shown = share;
        aiProgress.next({ ...aiProgress.value, subject: share });
      }, 80);
      try {
        subject = await edgeMatteAsync(mine as Parameters<typeof edgeMatteAsync>[0], rough as Parameters<typeof edgeMatteAsync>[1], state);
        if (subject) temporary.push(subject);
      } finally {
        clearInterval(poll);
      }
    }
    if (session !== preloadSession()) return undefined;
    phase('refined the edges');
    // The rough cutout is the result when the refiner is not there, so it is left out of what is freed.
    const result = subject ?? rough;
    const masks: Record<string, MaskLike> = { subject: result };
    if (missingModels([SKIN_SEGMENTATION]).length === 0) {
      phase('preparing the body parts');
      const parts = await fitBodyPartsAsync(
        mine as Parameters<typeof fitBodyPartsAsync>[0],
        result as Parameters<typeof fitBodyPartsAsync>[1],
        PARTS_MARGIN,
        PARTS_FOLLOW,
        0.002,
        state,
      );
      if (session !== preloadSession()) return undefined;
      phase('fitted the body parts');
      if (parts.length === 5) {
        const [whole, hair, skin, clothes, accessories] = parts as MaskLike[];
        temporary.push(...parts);
        masks.subject = whole;
        masks.hair = hair;
        masks.skin = skin;
        masks.clothes = clothes;
        masks.accessories = accessories;
        phase('followed the edges and fitted the parts');
      }
    }
    aiProgress.next({ ...aiProgress.value, subject: 1 });
    const transferred = new Set(Object.values(masks));
    temporary.splice(0, temporary.length, ...temporary.filter(mask => !transferred.has(mask)));
    return masks;
  } catch (error) {
    stopWatching();
    const { subject: _failed, ...others } = aiProgress.value;
    aiProgress.next(others);
    throw error;
  } finally {
    stopWatching();
    temporary.forEach(destroy);
    release(state);
    release(mine);
  }
};
