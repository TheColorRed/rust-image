import { AiModel } from '@abra/core';
import { rectIntersect, rectNew, rectRoundedOut, rectScaled, rectSquared, rectTranslated, type Rect } from '@abra/core/geometry';
import { type ImageLike } from '@abra/core/image';
import { aiModelPath } from '@alakazam/mobile';

export const PERSON_DETECTION = 'person-detection';
export const FACE_PARSING = 'face-parsing';
export const FACE_LANDMARKS = 'face-landmarks';
/** The AI models that finding a face needs, by role. */
export const FACE_MODELS = [PERSON_DETECTION, FACE_PARSING, FACE_LANDMARKS] as const;

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

const models: Record<string, ReturnType<typeof AiModel.load>> = {};

/** A model by its role, or nothing when it is not downloaded on this device (or has no manifest yet). */
export const model = (role: string): ReturnType<typeof AiModel.load> | undefined => {
  try {
    return (models[role] ??= AiModel.load(aiModelPath(role)));
  } catch {
    return undefined;
  }
};

/** The part of `box` that is on whole pixels inside `frame`, or nothing when it is smaller than `side` across. */
export const inside = (box: Rect, frame: Rect, side = MIN_CROP_SIDE): Rect | undefined => {
  const part = rectIntersect(rectRoundedOut(box), frame);
  return part.width >= side && part.height >= side ? part : undefined;
};

/** Runs `work`, then frees everything it kept, as the editor does for its images. Whatever the library hands back is kept. */
export function owning<T>(work: (keep: <V>(value: V) => V) => T): T {
  const owned: unknown[] = [];
  try {
    return work((value) => (owned.push(value), value));
  } finally {
    owned.forEach((value) => (value as { uniffiDestroy(): void }).uniffiDestroy());
  }
}

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
 * Names a face by the box around its person, as fractions of the photo, so that the name is the same at every size of the photo
 * and the masks made for one size can be resized for another.
 */
export const boxKey = (body: Rect, photo: ImageLike): string =>
  [body.x / photo.width(), body.y / photo.height(), body.width / photo.width(), body.height / photo.height()]
    .map(fraction => fraction.toFixed(5))
    .join(',');

/**
 * Whether the face named by `box` (from `boxKey`) is of `person`, a box in pixels of `photo`: its person's middle is inside it.
 * With no `person` it is everyone's.
 */
export const belongsTo = (
  box: string,
  photo: ImageLike,
  person: { x: number; y: number; width: number; height: number } | undefined,
): boolean => {
  if (!person) return true;
  const [x, y, width, height] = box.split(',').map(Number);
  const middleX = (x + width / 2) * photo.width();
  const middleY = (y + height / 2) * photo.height();
  return (
    middleX >= person.x && middleX <= person.x + person.width && middleY >= person.y && middleY <= person.y + person.height
  );
};
