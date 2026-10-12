import { channelMix } from '@abra/core/adjustments';
import { AiModel } from '@abra/core';
import { Color } from '@abra/core/color';
import { gaussianBlur } from '@abra/core/filters';
import {
  Area,
  type Rect,
  pointFLerp,
  rectCenter,
  rectIntersect,
  rectNew,
  rectOrigin,
  rectRoundedOut,
  rectScaled,
  rectSquared,
  rectTranslated,
} from '@abra/core/geometry';
import { type Image as AbraPixels, type ImageLike } from '@abra/core/image';
import { Mask, type MaskLike } from '@abra/core/mask';
import { aiModelPath, type AbraImageLike } from '@alakazam/mobile';

const PERSON_DETECTION = 'person-detection';
const FACE_PARSING = 'face-parsing';
const FACE_LANDMARKS = 'face-landmarks';
/** The AI models automatic red-eye removal needs, by role. Fixing an eye by tapping it needs none. */
export const RED_EYE_MODELS = [PERSON_DETECTION, FACE_PARSING, FACE_LANDMARKS] as const;
const EYES = ['left eye', 'right eye'];
/** The face-parsing classes that only a face has. Skin is left out: a body has plenty of it. */
const FACE_FEATURES = ['left eyebrow', 'right eyebrow', 'left eye', 'right eye', 'nose', 'mouth', 'upper lip', 'lower lip'];
/** The mesh points of the two irises: the center, then four points around it. */
const IRIS_STARTS = [468, 473];
/**
 * How much of the iris the pupil takes up, across. The landmarks give the edge of the iris, but red-eye is in the pupil in
 * the middle, and the rest of the iris must not be touched.
 */
const PUPIL_OF_IRIS = 0.7;
/** How soft the edge of the corrected area is, as a fraction of the pupil's width, so it fades out and leaves no ring. */
const SOFT_EDGE = 0.12;
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
/** What counts as the red of a flash: how far from pure red in hue (degrees), how saturated and bright, and how soft the edge. */
const RED_HUE_RANGE = 25;
const RED_MIN_SATURATION = 0.35;
/** Low, because the rim of a red pupil is dark red and must be picked up too, or it is left as an outline. */
const RED_MIN_VALUE = 0.1;
const RED_FEATHER = 0.4;
/**
 * Fixing an eye by tapping it has no model to say where the eyes are, so the pupil is found by its color, and these limits were
 * tuned on the photos in `assets/red-eye`, which `packages/bindings/uniffi/tests/red_eye_eval.rs` runs the same algorithm on.
 * Keep the two in step, and run it after any change.
 *
 * A flash pupil is red and saturated, which skin and lashes are not, so a strict test (`TAP_CORE_*`) finds the pupil without
 * them. It looks in a window around the tap, as a fraction of the shorter side of the photo, trying a small window first and
 * larger ones if it finds nothing, so the tap does not have to land on the pupil. While the pupil-colored pixels run into a side
 * of the window there is more of them past it, so it looks again in a window around what it found, up to `TAP_REFINES` times.
 */
const TAP_WINDOWS = [0.02, 0.04, 0.07];
const TAP_REFINES = 4;
/** The hue of a pupil, in degrees: red leaning towards magenta (pupils are 336 to 360, skin next to the eye is 5 and up), and how far from it still counts. */
const TAP_CORE_HUE = 350;
const TAP_CORE_HUE_RANGE = 14;
const TAP_CORE_MIN_SATURATION = 0.7;
const TAP_CORE_MIN_VALUE = 0.2;
const TAP_CORE_FEATHER = 0.1;
/** Only pixels at least half selected count towards where the pupil is, so the faint edge of a feather does not stretch it. */
const TAP_BOX_THRESHOLD = 127;
/**
 * What a pupil is like, to tell it from other red things, such as eye makeup, lips or a red shirt, in a photo lit red:
 * - small: under this fraction of the shorter side of the photo (pupils measured: 2 to 5 percent),
 * - roundish: this wide for its height, at least and at most (pupils measured: 0.75 to 1.1),
 * - filled in: at least this fraction of its box is pupil-colored, a disc fills about 0.8 and a bright catchlight inside takes
 *   some away (pupils measured: 0.36 to 0.76), while patchy makeup and thin lips are 0.02 to 0.15.
 */
const TAP_MAX_PUPIL = 0.06;
const TAP_PUPIL_SHAPE = [0.6, 1.5];
const TAP_MIN_FILL = 0.25;
/** The last look is at a window this many times the pupil found, across, centered on it, so that all of it is seen. */
const TAP_REFINE_WINDOW = 1.6;
/**
 * The correction itself uses a looser test (`TAP_RIM_*`), so the darker rim and the pinkish center of the pupil are included,
 * but only inside a disc this many times as wide as the pupil found, and only once the pupil has passed the size and shape
 * checks above, which keep it off skin and eye makeup.
 */
const TAP_PUPIL_GROWTH = 1.35;
const TAP_RIM_HUE_RANGE = 22;
const TAP_RIM_MIN_SATURATION = 0.5;
const TAP_RIM_MIN_VALUE = 0.1;
const TAP_RIM_FEATHER = 0.3;
/**
 * The correction rewrites the color of the pupil instead of covering it. Red is replaced by what green and blue say it should
 * be, so the excess red is gone and each pixel keeps its own green and blue, and with them the detail of the pupil. Then the
 * pupil is brought down to a darker level, because a pupil is dark and the light left in green and blue would look glassy.
 * `PUPIL_LEVEL` is the fraction of its brightness the pupil keeps; red also becomes `RED_OF_AVERAGE` of the average of
 * green and blue, a little below it.
 */
const PUPIL_LEVEL = 0.6;
const RED_OF_AVERAGE = 0.8;

const models: Record<string, ReturnType<typeof AiModel.load>> = {};

/** A model by its role, or nothing when it is not downloaded on this device (or has no manifest yet). */
const model = (role: string): ReturnType<typeof AiModel.load> | undefined => {
  try {
    return (models[role] ??= AiModel.load(aiModelPath(role)));
  } catch {
    return undefined;
  }
};

/** The part of `box` that is on whole pixels inside `frame`, or nothing when it is smaller than `side` across. */
const inside = (box: Rect, frame: Rect, side = MIN_CROP_SIDE): Rect | undefined => {
  const part = rectIntersect(rectRoundedOut(box), frame);
  return part.width >= side && part.height >= side ? part : undefined;
};

/** Runs `work`, then frees everything it kept, as the editor does for its images. Whatever the library hands back is kept. */
function owning<T>(work: (keep: <V>(value: V) => V) => T): T {
  const owned: unknown[] = [];
  try {
    return work((value) => (owned.push(value), value));
  } finally {
    owned.forEach((value) => (value as { uniffiDestroy(): void }).uniffiDestroy());
  }
}

/**
 * The mask of where the pupils are, made soft. Blurring the edge makes the correction fade out instead of stopping at a ring;
 * adding the blurred mask to itself doubles it, so the correction is at full strength right up to the rim of the pupil and the
 * fade starts beyond it.
 * - `pupils`: Where the pupils are.
 * - `pupilWidth`: How wide a pupil is, in pixels, which sets how soft the edge is.
 */
function softened(pupils: MaskLike, pupilWidth: number, keep: <V>(value: V) => V): MaskLike {
  const edge = keep(pupils.toImage());
  keep(gaussianBlur(Math.max(1, pupilWidth * SOFT_EDGE))).apply(edge);
  const blurred = keep(Mask.fromImage(edge));
  return keep(blurred.added(blurred));
}

/** Rewrites the color of the pupils on `photo`: the mixer described at `PUPIL_LEVEL`, applied through `pupils`. */
function correct(photo: ImageLike, pupils: MaskLike, keep: <V>(value: V) => V) {
  const red = (PUPIL_LEVEL * RED_OF_AVERAGE) / 2;
  keep(keep(channelMix([0, red, red], [0, PUPIL_LEVEL, 0], [0, 0, PUPIL_LEVEL])).withMask(pupils)).apply(photo);
}

/**
 * Removes red-eye from every person in the photo, with the AI models. The library does all the work: the models find the
 * people, the faces, the eyes and the irises as masks, the red pixels are picked out by color, the masks are combined and
 * placed back on the whole photo, and the color is rewritten through the result. This only says which steps to take and what
 * counts as red. Returns how many faces were corrected: none when the models are missing or no face was found.
 */
export const removeRedEye = (photo: ImageLike): number => {
  const people = model(PERSON_DETECTION);
  const parser = model(FACE_PARSING);
  const landmarker = model(FACE_LANDMARKS);
  if (!people || !parser || !landmarker) return 0;

  return owning((keep) => {
    const frame = rectNew({ x: 0, y: 0 }, { width: photo.width(), height: photo.height() });
    const white = keep(Color.fromRgba(255, 255, 255, 255));
    let redEyes: MaskLike | undefined;
    let faces = 0;

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

        // The eyes the parser found, limited to a disc over the pupil of each iris the landmarks found, and to the pixels that are red.
        const eyesOnPhoto = keep(keep(labels.maskNamed(EYES)).placed(photo.width(), photo.height(), rectOrigin(zoom)));
        const eyesInFace = keep(eyesOnPhoto.cropped(face));
        const irises = keep(Mask.empty(face.width, face.height));
        let pupilWidth = 0;
        for (const start of IRIS_STARTS) {
          const [center, ...ring] = landmarks.landmarks.slice(start, start + 5).map((landmark) => landmark.point);
          if (!center || ring.length !== 4) continue;
          const disc = keep(Area.circleAround(center, ring.map((point) => pointFLerp(center, point, PUPIL_OF_IRIS))));
          irises.drawArea(disc, white, rectOrigin(disc.bounds()));
          pupilWidth = Math.max(pupilWidth, disc.bounds().width);
        }
        const red = keep(Mask.fromColorRange(faceImage, keep(Color.red()), RED_HUE_RANGE, RED_MIN_SATURATION, RED_MIN_VALUE, RED_FEATHER));
        const redPupils = keep(keep(eyesInFace.minimum(irises)).minimum(red));
        const soft = keep(softened(redPupils, pupilWidth, keep).minimum(eyesInFace));
        const here = keep(soft.placed(photo.width(), photo.height(), rectOrigin(face)));
        redEyes = redEyes ? keep(redEyes.maximum(here)) : here;
        faces++;
        break;
      }
    }

    if (redEyes) correct(photo, redEyes, keep);
    return faces;
  });
};

/**
 * Fixes the red-eye at a spot the user tapped, without any model. It finds the pupil by its color near the tap, then rewrites
 * the color of the red pixels inside a disc a little larger than the pupil. Returns whether it found a pupil there to fix.
 * - `photo`: The photo, changed in place.
 * - `x`, `y`: The tapped spot, in pixels of the photo.
 */
export const fixRedEyeAt = (photo: ImageLike, x: number, y: number): boolean =>
  owning((keep) => {
    const frame = rectNew({ x: 0, y: 0 }, { width: photo.width(), height: photo.height() });
    const white = keep(Color.fromRgba(255, 255, 255, 255));
    const red = keep(Color.red());
    const pupilColor = keep(Color.fromHsv(TAP_CORE_HUE, 1, 1));
    const shorter = Math.min(photo.width(), photo.height());
    /**
     * What a square around a spot holds of pupil-colored pixels: the box around them in pixels of the photo, whether they run
     * into a side of the square other than where it meets the edge of the photo (there is more of them past it), and how full
     * the box is. Nothing when there are none.
     */
    const look = (cx: number, cy: number, half: number) => {
      const area = inside(rectNew({ x: cx - half, y: cy - half }, { width: half * 2, height: half * 2 }), frame, 4);
      if (!area) return undefined;
      const crop = keep(photo.cropped(area.x, area.y, area.width, area.height));
      const core = keep(Mask.fromColorRange(crop, pupilColor, TAP_CORE_HUE_RANGE, TAP_CORE_MIN_SATURATION, TAP_CORE_MIN_VALUE, TAP_CORE_FEATHER));
      const box = core.bounds(TAP_BOX_THRESHOLD);
      if (!box) return undefined;
      const cut =
        (box.x <= 0 && area.x > 0) ||
        (box.y <= 0 && area.y > 0) ||
        (box.x + box.width >= area.width && area.x + area.width < photo.width()) ||
        (box.y + box.height >= area.height && area.y + area.height < photo.height());
      const fill = keep(core.cropped(box)).coverage(TAP_BOX_THRESHOLD);
      return { box: rectTranslated(box, area.x, area.y), cut, fill };
    };

    // The pupil near the tap: a small square around it first and larger ones if it finds nothing, then, while the pupil-colored
    // pixels run into the side of the square, a square around what was found, until the pupil sits wholly inside one.
    let current: ReturnType<typeof look>;
    for (const fraction of TAP_WINDOWS) {
      current = look(x, y, shorter * fraction);
      if (current) break;
    }
    for (let refine = 0; current?.cut && refine < TAP_REFINES; refine++) {
      const middle = rectCenter(current.box);
      current = look(middle.x, middle.y, (Math.max(current.box.width, current.box.height) * TAP_REFINE_WINDOW) / 2 + 2);
    }
    if (!current || current.cut) return false;
    // One more look, centered on the pupil, to get all of it.
    const middle = rectCenter(current.box);
    const centered = look(middle.x, middle.y, (Math.max(current.box.width, current.box.height) * TAP_REFINE_WINDOW) / 2);
    const found = centered && !centered.cut ? centered : current;

    const pupil = found.box;
    const ratio = pupil.width / pupil.height;
    if (Math.max(pupil.width, pupil.height) > shorter * TAP_MAX_PUPIL || ratio < TAP_PUPIL_SHAPE[0] || ratio > TAP_PUPIL_SHAPE[1]) return false;
    if (found.fill < TAP_MIN_FILL) return false;

    // The correction: the looser red inside a disc around the pupil, softened.
    const size = Math.max(pupil.width, pupil.height) * TAP_PUPIL_GROWTH;
    const area = inside(rectScaled(pupil, TAP_PUPIL_GROWTH * 1.5, TAP_PUPIL_GROWTH * 1.5), frame, 4);
    if (!area) return false;
    const crop = keep(photo.cropped(area.x, area.y, area.width, area.height));
    const rim = keep(Mask.fromColorRange(crop, red, TAP_RIM_HUE_RANGE, TAP_RIM_MIN_SATURATION, TAP_RIM_MIN_VALUE, TAP_RIM_FEATHER));
    const center = rectCenter(pupil);
    const disc = keep(Area.circle({ x: center.x - area.x, y: center.y - area.y }, size / 2));
    const discs = keep(Mask.empty(crop.width(), crop.height()));
    discs.drawArea(disc, white, rectOrigin(disc.bounds()));
    const pupils = keep(discs.minimum(rim));
    const soft = keep(softened(pupils, size, keep));
    correct(photo, keep(soft.placed(photo.width(), photo.height(), rectOrigin(area))), keep);
    return true;
  });

/** A red-eye fix as the history records it: one tapped eye, or the automatic removal. */
export type RedEyeChange = { type: 'red-eye'; x: number; y: number } | { type: 'red-eye-auto' };

/**
 * Makes a red-eye fix on one of the editor's images, in place. Returns whether it found anything to fix; when it did not,
 * the image is left as it was.
 */
export function applyRedEyeStep(image: AbraImageLike, change: RedEyeChange): boolean {
  const photo = image.toImage();
  try {
    const fixed = change.type === 'red-eye' ? fixRedEyeAt(photo, change.x, change.y) : removeRedEye(photo) > 0;
    if (fixed) image.replaceWith(photo);
    return fixed;
  } finally {
    // It is a native image the garbage collector cannot see the size of.
    (photo as AbraPixels).uniffiDestroy();
  }
}
