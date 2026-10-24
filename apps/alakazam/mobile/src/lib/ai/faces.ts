import { AiModel } from '@abra/core';
import { rectIntersect, rectNew, rectRoundedOut, rectScaled, rectSquared, rectTranslated, type Rect } from '@abra/core/geometry';
import { type ImageLike } from '@abra/core/image';
import { aiModelPath, runAiModelAsync, unloadAiModels } from '@alakazam/mobile';
import { debounceTime, filter } from 'rxjs/operators';
import { personIn } from '@/src/lib/ai/analysis';
import { owning, release } from '@/src/lib/native/native-handles';
import { aiProgress, aiProgress$ } from '@/src/state/ai-progress';
import { faceBoxes, people } from '@/src/state/analysis';

export const PERSON_DETECTION = 'person-detection';
export const FACE_PARSING = 'face-parsing';
export const FACE_LANDMARKS = 'face-landmarks';
/** The AI models that finding a face needs, by role. */
export const FACE_MODELS = [PERSON_DETECTION, FACE_PARSING, FACE_LANDMARKS] as const;
/** The skin model's role. It also finds the hair, over the whole photo and not only near a face. */
export const SKIN_SEGMENTATION = 'skin-segmentation';
/** The edge refinement model's role (ViTMatte). It is not needed: with it the hair's edge is decided by it, without it by color. */
export const EDGE_REFINEMENT = 'edge-refinement-small';
/** The AI models the makeup controls need, by role: the face models. */
export const MAKEUP_MODELS = FACE_MODELS;
/** The AI model the hair controls need, by role: the one that finds the hair. The edge refinement model is not needed. */
export const HAIR_MODELS = [SKIN_SEGMENTATION] as const;

/** The face-parsing classes that only a face has. Skin is left out: a body has plenty of it. */
const FACE_FEATURES = ['left eyebrow', 'right eyebrow', 'left eye', 'right eye', 'nose', 'mouth', 'upper lip', 'lower lip'];
const EYES = ['left eye', 'right eye'];
/** The smallest crop, in pixels, worth running a model on. */
const MIN_CROP_SIDE = 32;
/**
 * Finding a face is done from coarse to fine, because a face in a full-length photo is small and in a close-up it fills the
 * picture, and the face parser only works on a crop where the face is a good part of it. First look in a square at the top of
 * the person, trying small squares (as a fraction of the person's height) before larger ones, until one leads to a face.
 */
const HEAD_SQUARES = [0.18, 0.3, 0.5, 0.75, 1];
const HEAD_RISE = 0.05;
/** The landmark model's score, from 0 to 1, above which a crop is taken to hold a face. */
const MIN_FACE_PRESENCE = 0.5;
/** Then crop tightly around the eyes: this many times the width of the eyes, so the face fills the crop. */
const EYES_TO_CROP = 3.2;
/** The landmark model gets the box around the face features, grown by this much, as a square. */
const FACE_GROWTH = 1.8;
/** The outline drawn for a face is the box around its landmarks grown by this much, so it takes in the forehead and the cheeks. */
const OUTLINE_GROWTH = 1.5;
/** The outline is moved up by this fraction of its side, because the landmarks stop at the forehead and the top of the head is above. */
const OUTLINE_RISE = 0.05;

const models: Record<string, ReturnType<typeof AiModel.load>> = {};

/** A model by its role, or nothing when it is not downloaded on this device (or has no manifest yet). */
export const model = (role: string): ReturnType<typeof AiModel.load> | undefined => {
  try {
    return (models[role] ??= AiModel.load(aiModelPath(role)));
  } catch {
    return undefined;
  }
};

/**
 * Frees a model by its role, and with it the memory it holds (a large model keeps gigabytes of working memory after a run). It is loaded
 * again by `model` the next time it is wanted.
 */
export const unloadModel = (role: string): void => {
  release(models[role]);
  delete models[role];
};

/** Frees every model, in this file and in the library, and the memory they hold. Each is loaded again the next time it is used. */
export const unloadAllModels = (): void => {
  Object.keys(models).forEach(unloadModel);
  unloadAiModels();
};

/**
 * Frees every model when no AI task is running, which is when every task listed in `aiProgress` is done. A task that is running keeps
 * what it uses, because a model freed under a running task would crash it.
 */
export const unloadModelsWhenIdle = (): void => {
  if (Object.values(aiProgress.value).every(progress => progress === 1)) unloadAllModels();
};
// A model is loaded when an analysis needs it and let go of when the AI work is done, so the large models do not stay in memory. The
// wait lets a task that follows straight on start before they are freed and loaded again; work that runs steps one after another
// frees its models between the steps itself (see `buildMasks`).
aiProgress$
  .pipe(
    debounceTime(300),
    filter(tasks => Object.keys(tasks).length > 0),
  )
  .subscribe(unloadModelsWhenIdle);

/** The part of `box` that is on whole pixels inside `frame`, or nothing when it is smaller than `side` across. */
export const inside = (box: Rect, frame: Rect, side = MIN_CROP_SIDE): Rect | undefined => {
  const part = rectIntersect(rectRoundedOut(box), frame);
  return part.width >= side && part.height >= side ? part : undefined;
};

/** One face the models found: what the face parser labeled in a crop around it, and where that crop is on the photo. */
export type FoundFace = {
  /** The box around the whole person this face belongs to, in pixels of the photo. */
  body: Rect;
  /** The face-parsing labels of the `zoom` crop. */
  labels: ReturnType<ReturnType<NonNullable<ReturnType<typeof model>>['run']>['labels']>;
  /** The crop `labels` were made from, in pixels of the photo. */
  zoom: Rect;
  /** The square around the face features, in pixels of the photo. */
  face: Rect;
  /** The photo cropped to `face`. */
  faceImage: ImageLike;
  landmarks: ReturnType<ReturnType<NonNullable<ReturnType<typeof model>>['run']>['landmarks']>;
};

/**
 * The face of each person in the photo, with the AI models. Nothing when a model is missing or no face is found. What it yields
 * is kept by `keep`, so it is freed with everything else the caller keeps.
 */
export function* findFaces(photo: ImageLike, keep: <V>(value: V) => V): Generator<FoundFace> {
  const people = model(PERSON_DETECTION);
  const parser = model(FACE_PARSING);
  const landmarker = model(FACE_LANDMARKS);
  if (!people || !parser || !landmarker) return;

  const frame = rectNew({ x: 0, y: 0 }, { width: photo.width(), height: photo.height() });
  for (const person of keep(people.run(photo)).instances()) {
    const body = keep(person).bounds();

    // Try a small square at the top of the person first and larger ones after it, because a face in a full-length photo is
    // small and in a close-up it fills the picture. A square counts when it leads to a face the landmark model accepts.
    for (const fraction of HEAD_SQUARES) {
      const side = body.height * fraction;
      const head = inside(
        rectNew({ x: body.x + body.width / 2 - side / 2, y: body.y - side * HEAD_RISE }, { width: side, height: side }),
        frame,
      );
      if (!head) continue;

      // Coarse: the eyes the parser sees in this square.
      const seen = keep(keep(keep(parser.run(keep(photo.cropped(head.x, head.y, head.width, head.height)))).labels()).maskNamed(EYES)).bounds(0);
      if (!seen) continue;

      // Fine: a crop around the eyes where the face fills the picture, so the parser finds the features properly.
      const zoom = inside(rectScaled(rectSquared(rectTranslated(seen, head.x, head.y)), EYES_TO_CROP, EYES_TO_CROP), frame);
      if (!zoom) continue;
      const labels = keep(keep(parser.run(keep(photo.cropped(zoom.x, zoom.y, zoom.width, zoom.height)))).labels());
      const features = keep(labels.maskNamed(FACE_FEATURES)).bounds(0);
      const face =
        features && inside(rectTranslated(rectScaled(rectSquared(features), FACE_GROWTH, FACE_GROWTH), zoom.x, zoom.y), frame);
      if (!face) continue;

      const faceImage = keep(photo.cropped(face.x, face.y, face.width, face.height));
      const landmarks = keep(landmarker.run(faceImage)).landmarks();
      if (landmarks.presence < MIN_FACE_PRESENCE) continue;

      yield { body, labels, zoom, face, faceImage, landmarks };
      break;
    }
  }
}

/**
 * `findFaces` with every model run on a worker thread, so the app keeps drawing and answering touches while it looks. The steps are
 * the same, so a change to how a face is found goes in both.
 */
export async function* findFacesAsync(photo: ImageLike, keep: <V>(value: V) => V): AsyncGenerator<FoundFace> {
  const people = model(PERSON_DETECTION);
  const parser = model(FACE_PARSING);
  const landmarker = model(FACE_LANDMARKS);
  if (!people || !parser || !landmarker) return;
  const run = async (loaded: NonNullable<typeof people>, image: ImageLike) =>
    keep(await runAiModelAsync(loaded, image as Parameters<typeof runAiModelAsync>[1]));

  // The bar under the photo: moving while the people are found, then how many of them have been looked at.
  aiProgress.next({ ...aiProgress.value, faces: null });
  let failed = false;
  try {
    const frame = rectNew({ x: 0, y: 0 }, { width: photo.width(), height: photo.height() });
    const found = (await run(people, photo)).instances();
    for (const [index, person] of found.entries()) {
      aiProgress.next({ ...aiProgress.value, faces: index / found.length });
      const body = keep(person).bounds();

      for (const fraction of HEAD_SQUARES) {
        const side = body.height * fraction;
        const head = inside(
          rectNew({ x: body.x + body.width / 2 - side / 2, y: body.y - side * HEAD_RISE }, { width: side, height: side }),
          frame,
        );
        if (!head) continue;

        const seen = keep(keep(keep(await run(parser, keep(photo.cropped(head.x, head.y, head.width, head.height)))).labels()).maskNamed(EYES)).bounds(0);
        if (!seen) continue;

        const zoom = inside(rectScaled(rectSquared(rectTranslated(seen, head.x, head.y)), EYES_TO_CROP, EYES_TO_CROP), frame);
        if (!zoom) continue;
        const labels = keep((await run(parser, keep(photo.cropped(zoom.x, zoom.y, zoom.width, zoom.height)))).labels());
        const features = keep(labels.maskNamed(FACE_FEATURES)).bounds(0);
        const face =
          features && inside(rectTranslated(rectScaled(rectSquared(features), FACE_GROWTH, FACE_GROWTH), zoom.x, zoom.y), frame);
        if (!face) continue;

        const faceImage = keep(photo.cropped(face.x, face.y, face.width, face.height));
        const landmarks = (await run(landmarker, faceImage)).landmarks();
        if (landmarks.presence < MIN_FACE_PRESENCE) continue;

        yield { body, labels, zoom, face, faceImage, landmarks };
        break;
      }
    }
  } catch (error) {
    failed = true;
    throw error;
  } finally {
    // A search that failed did no work to count, so it leaves nothing on the bar.
    const { faces: _earlier, ...others } = aiProgress.value;
    aiProgress.next(failed ? others : { ...aiProgress.value, faces: 1 });
  }
}

/**
 * Names a face by the box around its person, as fractions of the photo, so that the name is the same at every size of the photo
 * and the masks made for one size can be resized for another.
 */
export const boxKey = (body: Rect, photo: ImageLike): string =>
  [body.x / photo.width(), body.y / photo.height(), body.width / photo.width(), body.height / photo.height()]
    .map(fraction => fraction.toFixed(5))
    .join(',');

let findingFaceBoxes: Promise<void> | undefined;

/**
 * Finds the face of each person in `photo` with the face models, in the background, unless that has been done (it is saved with the
 * project, so it is found once per photo), and keeps its square in `faceBoxes`, in the pixels the people were found in. Each face goes
 * to the person whose box has the face's person's middle in it. Does nothing until the people are known. A model that is missing
 * leaves `faceBoxes` unset, so it is found when the models are there; no face found at all is kept, as an empty set.
 */
export const findFaceBoxes = (photo: ImageLike): Promise<void> => {
  const found = people.value;
  if (!found || faceBoxes.value || FACE_MODELS.some(role => !model(role))) return Promise.resolve();
  // Opening a person control again while it is still looking does not start a second search.
  findingFaceBoxes ??= (async () => {
    // The editor frees its own copy of the photo when the session ends, which can be before the models are done.
    const mine = photo.clone();
    const boxes: Record<number, Rect> = {};
    const started = Date.now();
    try {
      await owning(async keep => {
        for await (const { body, face: crop, landmarks } of findFacesAsync(mine, keep)) {
          // The landmarks cover the whole face, so the box around them is the face even when it is turned or tilted. The landmark
          // model draws its mesh inside the crop it is given, so a crop cut too tight gives a mesh that is too small: look again in a
          // crop made around the first mesh, and keep the second look when the landmark model accepts it.
          const bounds = (mesh: typeof landmarks, area: Rect) => {
            const xs = mesh.landmarks.map(landmark => landmark.point.x);
            const ys = mesh.landmarks.map(landmark => landmark.point.y);
            return {
              x: area.x + (Math.min(...xs) + Math.max(...xs)) / 2,
              y: area.y + (Math.min(...ys) + Math.max(...ys)) / 2,
              side: Math.max(Math.max(...xs) - Math.min(...xs), Math.max(...ys) - Math.min(...ys)),
            };
          };
          let { x, y, side } = bounds(landmarks, crop);
          const wider = inside(
            rectNew({ x: x - side, y: y - side }, { width: side * 2, height: side * 2 }),
            rectNew({ x: 0, y: 0 }, { width: mine.width(), height: mine.height() }),
          );
          const landmarker = model(FACE_LANDMARKS);
          if (wider && landmarker) {
            const again = keep(
              await runAiModelAsync(landmarker, keep(mine.cropped(wider.x, wider.y, wider.width, wider.height)) as Parameters<typeof runAiModelAsync>[1]),
            ).landmarks();
            if (again.presence >= MIN_FACE_PRESENCE) ({ x, y, side } = bounds(again, wider));
          }
          side *= OUTLINE_GROWTH;
          const face = { x: x - side / 2, y: y - side / 2 - side * OUTLINE_RISE, width: side, height: side };
          const middle = { x: body.x + body.width / 2, y: body.y + body.height / 2 };
          const owner = found.find(person => {
            const area = personIn(mine.width(), person.id)!;
            return middle.x >= area.x && middle.x <= area.x + area.width && middle.y >= area.y && middle.y <= area.y + area.height;
          });
          if (!owner || boxes[owner.id]) continue;
          const scale = owner.mask.width() / mine.width();
          boxes[owner.id] = { x: face.x * scale, y: face.y * scale, width: face.width * scale, height: face.height * scale };
        }
      });
      if (__DEV__) console.debug('[faces] face boxes found in the background, ms:', Date.now() - started, 'faces:', Object.keys(boxes).length);
      // The photo may have been closed while the models ran, and then what they found is for a photo that is gone.
      if (people.value === found) faceBoxes.next(boxes);
    } finally {
      release(mine);
      findingFaceBoxes = undefined;
    }
  })();
  return findingFaceBoxes;
};

/**
 * Whether the face named by `box` (from `boxKey`) is of `person`, a box in pixels of `photo`: its person's middle is inside it.
 * With no `person` it is everyone's, except the faces of the `others` (ids of people with a value of their own).
 */
export const belongsTo = (
  box: string,
  photo: { width(): number; height(): number },
  person: { x: number; y: number; width: number; height: number } | undefined,
  others: number[] = [],
): boolean => {
  const [x, y, width, height] = box.split(',').map(Number);
  const middleX = (x + width / 2) * photo.width();
  const middleY = (y + height / 2) * photo.height();
  const inside = (area: { x: number; y: number; width: number; height: number }) =>
    middleX >= area.x && middleX <= area.x + area.width && middleY >= area.y && middleY <= area.y + area.height;
  // The people with a value of their own are left out of what is for everyone.
  if (others.some(id => { const area = personIn(photo.width(), id); return !!area && inside(area); })) return false;
  return !person || inside(person);
};
