import { BodyRegion } from '@abra/core';
import { guidedFilter } from '@abra/core/filters';
import { type ImageLike } from '@abra/core/image';
import { TransformAlgorithm, type MaskLike } from '@abra/core/mask';
import { parseBodyWithinAsync, personDetectionDownloaded } from '@alakazam/mobile';
import { detectPeople } from '@/src/lib/ai/analysis';
import { edgeMatte } from '@/src/lib/ai/edge-matte';
import { EDGE_REFINEMENT, SKIN_SEGMENTATION, model, unloadModel } from '@/src/lib/ai/faces';
import { aiProgress } from '@/src/state/ai-progress';

/** How much of the photo around the people the body model is given, as a share of their size: a person who is small in the photo is only a few pixels to a model that sees a small copy. */
const PARTS_MARGIN = 0.1;
/** How far each part follows the edges of the photo, as a share of the people's size. */
const PARTS_FOLLOW = 0.012;

const destroy = (mask: MaskLike) => (mask as unknown as { uniffiDestroy(): void }).uniffiDestroy();

/**
 * The cutout of the people in a photo, as a mask of the whole photo (`subject`): white on the people, black elsewhere. With the body model on
 * this device it also gives the parts of the people, each followed to the edges of the photo and kept inside the cutout: `hair`, `skin`,
 * `clothes` and `accessories`. The person finder gives a
 * rough mask for each person, which are joined, and the edge refinement model, when it is on this device, decides the pixels along their
 * edge, hair included. Without that model the rough masks are the cutout. Nothing when the person finder is not on this device or finds
 * nobody, which a caller answers by not using a cutout. It runs in the background as a `preload`, once for a photo.
 */
export const subjectMask = async (photo: ImageLike): Promise<Record<string, MaskLike> | undefined> => {
  if (!personDetectionDownloaded()) return undefined;
  const mine = photo.clone();
  const temporary: MaskLike[] = [];
  aiProgress.next({ ...aiProgress.value, subject: null });
  try {
    // The people are found once for the photo and shared, so their masks are not freed here.
    const found = await detectPeople(mine as Parameters<typeof detectPeople>[0], { silent: true });
    let rough: MaskLike | undefined;
    for (const person of found) {
      const sized = person.mask.resized(mine.width(), mine.height(), TransformAlgorithm.Bilinear);
      temporary.push(sized);
      rough = rough ? rough.maximum(sized) : sized;
      if (rough !== sized) temporary.push(rough);
    }
    if (!rough?.bounds(0)) {
      aiProgress.next({ ...aiProgress.value, subject: 1 });
      return undefined;
    }
    let subject: MaskLike | undefined;
    const refiner = model(EDGE_REFINEMENT);
    if (refiner) {
      try {
        const keep = <V>(value: V): V => {
          temporary.push(value as unknown as MaskLike);
          return value;
        };
        subject = await edgeMatte(refiner, mine, rough, keep, share =>
          aiProgress.next({ ...aiProgress.value, subject: share }),
        );
      } finally {
        // The model holds gigabytes of memory after it has run, and it runs once per photo, so it is not kept.
        unloadModel(EDGE_REFINEMENT);
      }
    }
    // The rough cutout is the result when the refiner is not there, so it is left out of what is freed.
    const result = subject ?? rough;
    if (!subject) temporary.splice(temporary.indexOf(rough), 1);
    const masks: Record<string, MaskLike> = { subject: result };
    if (model(SKIN_SEGMENTATION)) {
      const box = result.bounds(128);
      if (box) {
        const parts = await parseBodyWithinAsync(
          mine as Parameters<typeof parseBodyWithinAsync>[0],
          box.x,
          box.y,
          box.width,
          box.height,
          PARTS_MARGIN,
        );
        temporary.push(parts as unknown as MaskLike);
        // The model's edges are rough, so each part is made to follow the edges of the photo, as far as is wanted of a person this big.
        const radius = Math.max(2, Math.round(Math.sqrt(box.width * box.height) * PARTS_FOLLOW));
        const regions: Record<string, MaskLike> = {
          hair: parts.mask(BodyRegion.Hair),
          skin: parts.maskOf([BodyRegion.BodySkin, BodyRegion.FaceSkin]),
          clothes: parts.mask(BodyRegion.Clothes),
          accessories: parts.mask(BodyRegion.Accessories),
        };
        for (const [name, region] of Object.entries(regions)) {
          temporary.push(region);
          const followed = guidedFilter(region, mine, radius, 0.002);
          temporary.push(followed);
          masks[name] = followed.multiplied(result);
        }
      }
    }
    aiProgress.next({ ...aiProgress.value, subject: 1 });
    return masks;
  } catch (error) {
    const { subject: _failed, ...others } = aiProgress.value;
    aiProgress.next(others);
    throw error;
  } finally {
    temporary.forEach(destroy);
    (mine as unknown as { uniffiDestroy(): void }).uniffiDestroy();
  }
};
