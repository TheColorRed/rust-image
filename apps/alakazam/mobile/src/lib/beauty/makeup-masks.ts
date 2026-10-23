import { gaussianBlur } from '@abra/core/filters';
import { rectOrigin } from '@abra/core/geometry';
import { type ImageLike } from '@abra/core/image';
import { Mask, type MaskLike } from '@abra/core/mask';
import { type ToolContext } from '@/src/lib/project/edit-sections';
import { MAKEUP_MODELS, boxKey, findFacesAsync, model } from '@/src/lib/ai/faces';
import { owning, release } from '@/src/lib/native/native-handles';

const LIPS = ['upper lip', 'lower lip'];

/** How much of a skin edit the lips and the mouth get: half, so they match the face without being as strong as the skin. */
const MOUTH_SHARE = 0.5;
/** How wide the fade from the skin into the mouth is, as a fraction of the mouth's width. */
const MOUTH_FADE = 0.12;

/**
 * `skin` with the lips and the mouth of the faces in `preloaded` (what `makeupMasks` made) weaker, for a skin edit that would
 * otherwise tint them as strongly as the skin: they get `MOUTH_SHARE` of it, faded in softly at the edge. Anything made is handed to
 * `keep`; `skin` itself is not changed.
 *
 * `ease` is how much of `skin` is taken off where a part is (1 takes it all off), and `parts` are the names in `preloaded` that start
 * a part to take it off: the lips and the mouth by default; `eye|` and `brow|` are the eyes and the eyebrows. A skin edit that
 * replaces color, and so would show on the lips, the whites of the eyes and the eyebrows, takes all of them off.
 */
export const skinWithMouthEased = (
  skin: MaskLike,
  preloaded: ToolContext['preloaded'],
  keep: <T>(item: T) => T,
  ease = MOUTH_SHARE,
  parts = ['lipstick|', 'mouth|'],
): MaskLike => {
  if (!preloaded || Array.isArray(preloaded)) return skin;
  let eased = skin;
  const half = keep(Mask.fromValues(skin.width(), skin.height(), new Uint8Array(skin.width() * skin.height()).fill(Math.round(255 * ease)).buffer));
  for (const [key, part] of Object.entries(preloaded)) {
    if (!parts.some(start => key.startsWith(start))) continue;
    const box = (part as MaskLike).bounds(0);
    if (!box) continue;
    const blurred = keep((part as MaskLike).toImage());
    keep(gaussianBlur(box.width * MOUTH_FADE)).apply(blurred);
    eased = keep(eased.subtracted(keep(keep(Mask.fromImage(blurred)).multiplied(half))));
  }
  return eased;
};

/**
 * Runs face analysis once and keeps what every region of the Makeup group is made from: the lips, and the eye and eyebrow of each
 * side, named `lipstick|person`, `skin|person`, `nose|person`, `eye|person|side` and `brow|person|side` (the person is the box around them, as fractions of the photo: see `boxKey`), The hair is not here: it has its own analysis (`hairMasks`). The regions
 * themselves are made when the color is applied, because their size is a slider.
 *
 * The models run in the background, so this gives back a promise and the editor keeps going meanwhile: a control that needs the masks
 * paints without them until they are made, and the editor draws again when they are. The photo is copied before anything is awaited,
 * because the editor frees the one it passed as soon as this returns.
 */
export const makeupMasks = async (photo: ImageLike): Promise<Record<string, MaskLike> | undefined> => {
  if (MAKEUP_MODELS.some(role => !model(role))) return undefined;
  const mine = photo.clone();
  const made: Record<string, MaskLike> = {};
  try {
    await owning(async keep => {
      for await (const { body, labels, zoom } of findFacesAsync(mine, keep)) {
        const box = boxKey(body, mine);
        const place = (mask: MaskLike) => mask.placed(mine.width(), mine.height(), rectOrigin(zoom));
        const lips = keep(labels.maskNamed(LIPS));
        if (lips.bounds(0)) made[`lipstick|${box}`] = place(lips);
        const mouth = keep(labels.maskNamed(['mouth']));
        if (mouth.bounds(0)) made[`mouth|${box}`] = place(mouth);
        const skin = keep(labels.maskNamed(['skin']));
        if (skin.bounds(0)) made[`skin|${box}`] = place(skin);
        const nose = keep(labels.maskNamed(['nose']));
        if (nose.bounds(0)) made[`nose|${box}`] = place(nose);
        for (const side of ['left', 'right'] as const) {
          const eye = keep(labels.maskNamed([`${side} eye`]));
          if (!eye.bounds(0)) continue;
          made[`eye|${box}|${side}`] = place(eye);
          const brow = keep(labels.maskNamed([`${side} eyebrow`]));
          if (brow.bounds(0)) made[`brow|${box}|${side}`] = place(brow);
        }
      }
    });
  } catch (error) {
    // A model that failed part of the way leaves nothing behind.
    Object.values(made).forEach(mask => release(mask));
    throw error;
  } finally {
    release(mine);
  }
  // No masks is still an answer (no face was found), so it is kept and saved and the models are not run for this photo again.
  return made;
};

