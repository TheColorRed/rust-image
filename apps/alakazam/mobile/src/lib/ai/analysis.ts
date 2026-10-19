import { TransformAlgorithm, type MaskLike } from '@abra/core/mask';
import { type Image as AbraPixels } from '@abra/core/image';
import { type PersonDetection } from '@alakazam/mobile';
import { detectSkinAsync } from '@alakazam/mobile';
import { aiProgress } from '@/src/state/ai-progress';
import { faceBoxes, people, skinMask } from '@/src/state/analysis';
import { adjustments } from '@/src/state/edits';
import { personSelection } from '@/src/state/person-selection';
import type { Spot } from '@/src/lib/project/timeline';

// The masks made from the skin and the people, kept until either changes: making one takes as long as a pass over the photo.
let made: { skin: MaskLike | null; list: PersonDetection[] | null; full: Map<number | null, MaskLike | undefined>; sized: Map<string, MaskLike> } | undefined;

const cache = () => {
  if (made?.skin !== skinMask.value || made.list !== people.value) {
    made = { skin: skinMask.value, list: people.value, full: new Map(), sized: new Map() };
  }
  return made;
};

/**
 * The mask of the skin of the person with this id, or of everyone, at the size of the photo the skin was found in. It is nothing
 * while the skin has not been found. The same mask is handed out until the skin or the people change, so do not free it.
 */
export const skinMaskOf = (personId: number | null | undefined): MaskLike | undefined => {
  const { full, skin, list } = cache();
  const id = personId ?? null;
  if (!full.has(id)) {
    const person = id === null ? undefined : list?.find(found => found.id === id);
    full.set(id, person ? (skin ? person.mask.multiplied(skin) : person.mask) : (skin ?? undefined));
  }
  return full.get(id);
};

/**
 * The mask of the skin of the person with this id, or of everyone, resized to `width` x `height`. It is nothing while the skin
 * has not been found. The caller frees it: it is a copy of one that is kept for the next call at this size.
 */
export const skinMaskSized = (
  personId: number | null | undefined,
  width: number,
  height: number,
  except: number[] = [],
): MaskLike | undefined => {
  const mask = skinMaskOf(personId);
  if (!mask) return undefined;
  const { sized } = cache();
  const key = `${personId ?? ''}:${width}x${height}`;
  let held = sized.get(key);
  if (!held) {
    if (sized.size >= 8) sized.clear();
    sized.set(key, (held = mask.resized(width, height, TransformAlgorithm.Bilinear)));
  }
  let result = held.resized(width, height, TransformAlgorithm.Bilinear);
  // The people with a value of their own are left out, so theirs replaces this one for them.
  for (const id of except) {
    const person = people.value?.find(found => found.id === id);
    if (!person) continue;
    const there = person.mask.resized(width, height, TransformAlgorithm.Bilinear);
    const rest = result.subtracted(there);
    (there as unknown as { uniffiDestroy(): void }).uniffiDestroy();
    (result as unknown as { uniffiDestroy(): void }).uniffiDestroy();
    result = rest;
  }
  return result;
};

/** The mask of the skin the effects work on: the selected person's, or everyone's when nobody is selected. Do not free it. */
export const selectedSkinMask = (): MaskLike | undefined => skinMaskOf(personSelection.value.selectedId);

/** The box of the person with this id, in the pixels of a photo that is `width` wide. Nothing when there is no such person. */
export const personIn = (width: number, id: number | null | undefined) => {
  const person = id == null ? undefined : people.value?.find(found => found.id === id);
  if (!person) return undefined;
  const scale = width / person.mask.width();
  return {
    id: person.id,
    x: person.x * scale,
    y: person.y * scale,
    width: person.width * scale,
    height: person.height * scale,
    confidence: person.confidence,
  };
};

/**
 * The person whose body is at this point of a photo of this size, or nothing. Where bodies overlap it is the one the mask is
 * surest of there.
 */
export const personAt = (x: number, y: number, width: number, height: number): number | undefined => {
  let found: PersonDetection | undefined;
  let surest = 0;
  for (const person of people.value ?? []) {
    const value = person.mask.get(
      Math.floor((x * person.mask.width()) / width),
      Math.floor((y * person.mask.height()) / height),
    );
    if (value !== undefined && (value > surest || (value === surest && value > 0 && person.confidence > (found?.confidence ?? 0)))) {
      surest = value;
      found = person;
    }
  }
  return found?.id;
};

/**
 * The people who are still in the photo after the edits that replace content (see `replacedSpots`): a person with under a quarter
 * of their body outside the replaced spots has been removed from the picture, so they are no longer there to pick. The people are
 * found once, in the original, so this is worked out from the edits and nothing is saved. `spots` are in the pixels of a photo
 * that is `photoWidth` wide.
 */
export const peopleNotHealed = (
  list: PersonDetection[],
  spots: Spot[],
  photoWidth: number,
): PersonDetection[] => {
  if (spots.length === 0) return list;
  return list.filter(person => {
    const width = person.mask.width();
    const values = new Uint8Array(person.mask.values());
    const scale = width / photoWidth;
    let total = 0;
    let left = 0;
    for (let index = 0; index < values.length; index++) {
      if (values[index] <= 127) continue;
      total++;
      const x = index % width;
      const y = Math.floor(index / width);
      if (!spots.some(spot => (x - spot.x * scale) ** 2 + (y - spot.y * scale) ** 2 <= (spot.radius * scale) ** 2)) left++;
    }
    return total === 0 || left >= total * 0.25;
  });
};

let detecting: Promise<void> | undefined;

/**
 * Finds the skin of the photo with the skin model, unless it has been found, and redraws the edits that use it. This is the only
 * way the skin model runs: when a control that needs the skin is used, never when a photo opens. Fails if the model is not on
 * the device.
 */
export const detectSkin = (photo: Parameters<typeof detectSkinAsync>[0]): Promise<void> => {
  if (skinMask.value) return Promise.resolve();
  aiProgress.next({ ...aiProgress.value, skin: null });
  detecting ??= detectSkinAsync(photo)
    .then(mask => {
      aiProgress.next({ ...aiProgress.value, skin: 1 });
      (skinMask.value as unknown as AbraPixels | null)?.uniffiDestroy();
      skinMask.next(mask);
      adjustments.next({ ...adjustments.value });
    })
    .catch(error => {
      // A model that could not run (it is not on the device) did no work, so it leaves nothing on the bar.
      const { skin: _failed, ...others } = aiProgress.value;
      aiProgress.next(others);
      throw error;
    })
    .finally(() => {
      detecting = undefined;
    });
  return detecting;
};

/** Forgets the skin and the people of the photo that was open, and frees their masks. */
export const clearAnalysis = () => {
  const skin = skinMask.value;
  const list = people.value;
  made = undefined;
  skinMask.next(null);
  people.next(null);
  faceBoxes.next(null);
  (skin as unknown as AbraPixels | null)?.uniffiDestroy();
  for (const person of list ?? []) (person.mask as unknown as AbraPixels).uniffiDestroy();
};
