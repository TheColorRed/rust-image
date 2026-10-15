import { Color } from '@abra/core/color';
import { gaussianBlur } from '@abra/core/filters';
import { rectOrigin } from '@abra/core/geometry';
import { type ImageLike } from '@abra/core/image';
import { Mask, TransformAlgorithm, type MaskLike } from '@abra/core/mask';
import { type ToolContext } from '@/src/lib/edit-sections';
import { derivedFor } from '@/src/lib/editor/editor-preloads';
import { paintSheet, type Painting } from '@/src/lib/layers';
import { FACE_MODELS, belongsTo, boxKey, findFacesAsync, model, owning } from '@/src/lib/faces';

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

type EyeMakeup = 'eyeliner' | 'eyeshadow' | 'brow-bone' | 'under-eye';

/**
 * How many widths of one eye the two eyes span. The softness of a region was tuned against the width of both eyes together, so
 * it is measured in these now that each eye is made on its own.
 */
const EYES_SPAN = 2.6;

/**
 * The widest photo a region is made on. Making one takes a crop around the eye, a blur as wide as the region is far-reaching and
 * a few passes over the crop, and on a full-size photo that is a very large crop and blur for a soft shape. The region is made
 * on a copy of the eye no wider than this and resized up to the photo, which is the same shape.
 */
const WORK_WIDTH = 1280;

/**
 * The fullest a region can be, and the least. A region is made once at the fullest, as a map of how near each pixel is to the eye
 * (see `eyeField`), and the fullness slider only chooses how much of the map is shown, so moving it makes no shape. An eyeliner's
 * fullness is its width.
 */
const FULL_MAX = 3;
const FULL_MIN = 0.3;

/** How sharply the edge of the shown part of the map falls off, as a multiple of the map's own steepness. */
const EDGE_GAIN = 2;

/** How far towards the middle of the face (the nose bridge) a region may reach from its eye: this fraction of the way. */
const TOWARDS_MIDDLE = 0.75;

/** A vertical band with feathered top and bottom edges, used to avoid a visible crop line across the eyelid. */
const featheredBand = (width: number, height: number, top: number, bottom: number, feather: number): MaskLike => {
  const values = new Uint8Array(width * height);
  const safeFeather = Math.max(1, feather);
  for (let y = 0; y < height; y += 1) {
    const fromTop = Math.min(1, Math.max(0, (y - top) / safeFeather));
    const fromBottom = Math.min(1, Math.max(0, (bottom - y) / safeFeather));
    values.fill(Math.round(255 * Math.min(fromTop, fromBottom)), y * width, (y + 1) * width);
  }
  return Mask.fromValues(width, height, values.buffer);
};

/**
 * Builds what the region around one eye is shown from, once: a map of how near each pixel is to the eye (full at its outline,
 * falling to nothing at the fullest reach), and a mask of everything that limits the region (the bands, the eyebrow, the nose).
 * Neither depends on the fullness, so they are made when the control opens and kept; `regionAt` shows as much of the map as the
 * fullness says. The limits are kept apart from the map so that showing more of it does not sharpen their soft edges. It is made
 * from the eye (and for the brow bone the eyebrow) the face parser found, so the region follows the eye at any fullness. It
 * stays on its own side of `midline` (the column halfway between the eyes, in pixels of the photo), so a full one never crosses the
 * nose to meet the other. The work is done on a crop around the eye and put back on the photo.
 */
const eyeField = (
  eyes: MaskLike,
  eyebrows: MaskLike | undefined,
  nose: MaskLike | undefined,
  kind: EyeMakeup,
  midline: number,
  keep: <V>(value: V) => V,
): { field: MaskLike; limits: MaskLike } | undefined => {
  const eyeBounds = eyes.bounds(0);
  if (!eyeBounds) return undefined;
  const fullest = FULL_MAX;

  // A crop around the eye with room for the fullest region.
  const reach = Math.max(eyeBounds.width, eyeBounds.height) * 3 * fullest;
  const left = Math.max(0, Math.floor(eyeBounds.x - reach));
  const top = Math.max(0, Math.floor(eyeBounds.y - reach));
  const right = Math.min(eyes.width(), Math.ceil(eyeBounds.x + eyeBounds.width + reach));
  const bottom = Math.min(eyes.height(), Math.ceil(eyeBounds.y + eyeBounds.height + reach));
  const crop = { x: left, y: top, width: right - left, height: bottom - top };

  const eye = keep(eyes.cropped(crop));
  const bounds = eye.bounds(0);
  if (!bounds) return undefined;
  const width = eye.width();
  const height = eye.height();

  // The eye spread as far as it goes, brought up so that its brightest part is full, and never on the eye itself.
  const blurredImage = keep(eye.toImage());
  keep(
    gaussianBlur(
      Math.max(
        1.5,
        bounds.width *
          EYES_SPAN *
          (kind === 'eyeliner' ? 0.03 : kind === 'eyeshadow' ? 0.12 : kind === 'brow-bone' ? 0.15 : 0.14) *
          fullest,
      ),
    ),
  ).apply(blurredImage);
  const peak = new Uint8Array(keep(Mask.fromImage(blurredImage)).values()).reduce((most, value) => Math.max(most, value), 1);
  blurredImage.mul(255 / peak);
  const field = keep(keep(Mask.fromImage(blurredImage)).subtracted(eye));

  let limits: MaskLike = keep(Mask.fromValues(width, height, new Uint8Array(width * height).fill(255).buffer));
  // The highest a region goes: just under the eyebrow, along its lower edge, so the color never runs up over the brow. In each
  // column, only below where the eyebrow ends (and a small gap), and softened. Columns past the eyebrow take the nearest column's
  // edge. A region that reaches down (the under eye) has no use for it.
  const brow = eyebrows && keep(eyebrows.cropped(crop));
  const hasBrow = !!brow && !!brow.bounds(0);
  if (hasBrow && (kind === 'eyeshadow' || kind === 'brow-bone')) {
    const browValues = new Uint8Array(brow.values());
    const bottoms = Array.from({ length: width }, (_, x) => {
      for (let y = height - 1; y >= 0; y -= 1) if (browValues[y * width + x] > 127) return y;
      return -1;
    });
    for (let x = 1; x < bottoms.length; x += 1) if (bottoms[x] < 0) bottoms[x] = bottoms[x - 1];
    for (let x = bottoms.length - 2; x >= 0; x -= 1) if (bottoms[x] < 0) bottoms[x] = bottoms[x + 1];
    const gap = Math.round(bounds.height * 0.25);
    const belowValues = new Uint8Array(width * height);
    for (let x = 0; x < bottoms.length; x += 1) {
      for (let y = Math.max(0, bottoms[x] + gap); y < height; y += 1) belowValues[y * width + x] = 255;
    }
    const belowImage = keep(keep(Mask.fromValues(width, height, belowValues.buffer)).toImage());
    keep(gaussianBlur(Math.max(1.5, bounds.height * 0.35))).apply(belowImage);
    limits = keep(limits.minimum(keep(Mask.fromImage(belowImage))));
  }
  if (kind === 'eyeshadow') {
    // Up to the brow, and a little below the lash line. Without an eyebrow to stop at, as far up as the fullness reaches.
    const bandTop = hasBrow ? 0 : Math.max(0, bounds.y - bounds.height * 1.1 * fullest);
    const bandBottom = Math.min(height, bounds.y + bounds.height * 0.45);
    limits = keep(
      limits.minimum(keep(featheredBand(width, height, bandTop, bandBottom, Math.max(2, bounds.height * 0.5)))),
    );
  } else if (kind === 'brow-bone') {
    if (!hasBrow) return undefined;
    // Only above the eye: in each column, down to where the eye starts. The bottom edge is the eye's own upper edge, so the
    // region arches with the eye and the lid under it stays clear. Columns past the corners take the nearest column's edge.
    const eyeValues = new Uint8Array(eye.values());
    const tops = Array.from({ length: width }, (_, x) => {
      for (let y = 0; y < height; y += 1) if (eyeValues[y * width + x] > 127) return y;
      return -1;
    });
    for (let x = 1; x < tops.length; x += 1) if (tops[x] < 0) tops[x] = tops[x - 1];
    for (let x = tops.length - 2; x >= 0; x -= 1) if (tops[x] < 0) tops[x] = tops[x + 1];
    const overlap = Math.round(bounds.height * 0.1);
    const aboveValues = new Uint8Array(width * height);
    for (let x = 0; x < tops.length; x += 1) {
      for (let y = 0; y < Math.min(height, tops[x] + overlap); y += 1) aboveValues[y * width + x] = 255;
    }
    const aboveImage = keep(keep(Mask.fromValues(width, height, aboveValues.buffer)).toImage());
    keep(gaussianBlur(Math.max(1.5, bounds.height * 0.3))).apply(aboveImage);
    limits = keep(limits.minimum(keep(Mask.fromImage(aboveImage))));
  } else if (kind === 'under-eye') {
    // From the lash line down.
    const bandTop = Math.max(0, bounds.y + bounds.height * 0.45);
    const bandBottom = Math.min(height, bounds.y + bounds.height * (0.45 + 1.7 * fullest));
    limits = keep(
      limits.minimum(keep(featheredBand(width, height, bandTop, bandBottom, Math.max(2, bounds.height * 0.55 * fullest)))),
    );
  }

  // Never over the nose: the nose the parser found, softened, is cut out of the region, so its inner edge follows the arch of the
  // nose and not only a line down the middle of the face.
  const noseHere = nose && keep(nose.cropped(crop));
  if (noseHere?.bounds(0)) {
    const noseImage = keep(noseHere.toImage());
    keep(gaussianBlur(Math.max(1.5, bounds.height * 0.4))).apply(noseImage);
    noseImage.mul(2);
    limits = keep(limits.subtracted(keep(Mask.fromImage(noseImage))));
  }

  // Keep it on this eye's side of the middle of the face, stopping short of it so the two eyes' regions never meet over the nose
  // bridge, with a soft edge so there is no line. Every row is the same, so one is made and copied down.
  const eyeMiddle = eyeBounds.x + eyeBounds.width / 2;
  const before = eyeMiddle < midline;
  const edge = eyeMiddle + (midline - eyeMiddle) * TOWARDS_MIDDLE - left;
  const soft = Math.max(2, eyeBounds.width * 0.6);
  const sideRow = new Uint8Array(width);
  for (let x = 0; x < width; x += 1) {
    sideRow[x] = Math.round(255 * Math.min(1, Math.max(0, (before ? edge - x : x - edge) / soft + 0.5)));
  }
  const sideValues = new Uint8Array(width * height);
  for (let y = 0; y < height; y += 1) sideValues.set(sideRow, y * width);
  limits = keep(limits.minimum(keep(Mask.fromValues(width, height, sideValues.buffer))));

  const at = { x: left, y: top };
  return { field: field.placed(eyes.width(), eyes.height(), at), limits: limits.placed(eyes.width(), eyes.height(), at) };
};

/**
 * The part of an eye's map that a fullness shows, as the region to put color through. A few passes over the map, with no shape to
 * make: all but the nearest part of the map is hidden at the least fullness, and none of it at the most.
 */
const regionAt = (
  field: MaskLike,
  limits: MaskLike,
  kind: EyeMakeup,
  fullness: number,
  keep: <V>(value: V) => V,
): MaskLike => {
  const reach = Math.min(1, Math.max(0, (fullness - FULL_MIN) / (FULL_MAX - FULL_MIN)));
  const image = keep(field.toImage());
  image.sub(255 * 0.85 * (1 - reach) ** 1.5);
  image.mul(EDGE_GAIN);
  return keep(Mask.fromImage(image)).minimum(limits);
};

/**
 * Runs face analysis once and keeps what every region of the Makeup group is made from: the lips, and the eye and eyebrow of each
 * side, named `lipstick|person`, `skin|person`, `nose|person`, `eye|person|side` and `brow|person|side` (the person is the box around them, as fractions of the photo: see `boxKey`). The regions
 * themselves are made when the color is applied, because their size is a slider.
 *
 * The models run in the background, so this gives back a promise and the editor keeps going meanwhile: a control that needs the masks
 * paints without them until they are made, and the editor draws again when they are. The photo is copied before anything is awaited,
 * because the editor frees the one it passed as soon as this returns.
 */
export const makeupMasks = async (photo: ImageLike): Promise<Record<string, MaskLike> | undefined> => {
  if (FACE_MODELS.some(role => !model(role))) return undefined;
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
    Object.values(made).forEach(mask => (mask as unknown as { uniffiDestroy(): void }).uniffiDestroy());
    throw error;
  } finally {
    (mine as unknown as { uniffiDestroy(): void }).uniffiDestroy();
  }
  // No masks is still an answer (no face was found), so it is kept and saved and the models are not run for this photo again.
  return made;
};

/**
 * Paints the color of a hue (0 to 1 along `Gradient.hue()`), as colorful as `saturation` and as bright as `brightness` (both 0 to
 * 1), around each eye of the person in `context` (or of everyone) on `sheet`, which becomes a layer over the photo. How much of
 * it shows is the layer's opacity, and how it mixes with the skin is the layer's blend. `fullness` is how far the color reaches out
 * from each eye (1 is as made; more covers more of the lid, brow bone or under eye).
 */
export const applyEyeMakeup = (
  kind: EyeMakeup,
  sheet: Painting,
  hue: number,
  saturation: number,
  brightness: number,
  fullness: number,
  { preloaded, person, others }: ToolContext,
) => {
  if (!preloaded || Array.isArray(preloaded)) return;
  owning(keep => {
    const color = keep(Color.fromHsv(360 * (1 - hue), saturation, brightness));
    const regions: MaskLike[] = [];
    for (const key of Object.keys(preloaded)) {
      const [maskKind, box, side] = key.split('|');
      if (maskKind !== 'eye' || !box || !side) continue;
      if (!belongsTo(box, sheet, person, others)) continue;
      // The map of how near each pixel is to the eye, and what limits it, are made once for this eye and kept: no slider changes them.
      // Regions are made at a working size, not the photo's (see `WORK_WIDTH`).
      const shrink = Math.min(1, WORK_WIDTH / sheet.width());
      let limits: MaskLike | undefined;
      const field = derivedFor(preloaded, `${kind}|${key}|field`, '', () =>
        owning(own => {
          const atWorkSize = (name: string): MaskLike | undefined => {
            const mask = preloaded[name] as MaskLike | undefined;
            if (!mask || shrink === 1) return mask;
            return own(
              mask.resized(Math.round(mask.width() * shrink), Math.round(mask.height() * shrink), TransformAlgorithm.Bilinear),
            );
          };
          const eye = atWorkSize(key);
          if (!eye) return undefined;
          // The middle of the face is halfway between this person's eyes.
          const eyeCenters = ['left', 'right'].flatMap(each => {
            const bounds = atWorkSize(`eye|${box}|${each}`)?.bounds(0);
            return bounds ? [bounds.x + bounds.width / 2] : [];
          });
          const midline = eyeCenters.length === 2 ? (eyeCenters[0] + eyeCenters[1]) / 2 : (sheet.width() * shrink) / 2;
          const made = eyeField(eye, atWorkSize(`brow|${box}|${side}`), atWorkSize(`nose|${box}`), kind, midline, own);
          limits = made?.limits;
          return made?.field;
        }),
      ) as MaskLike | undefined;
      const limitsHeld = derivedFor(preloaded, `${kind}|${key}|limits`, '', () => limits) as MaskLike | undefined;
      if (!field || !limitsHeld) continue;
      // Showing part of the map is a few passes over it, whatever the slider is.
      const region = owning(own => {
        const shown = regionAt(field, limitsHeld, kind, fullness, own);
        return shrink === 1 ? shown : own(shown).resized(sheet.width(), sheet.height(), TransformAlgorithm.Bilinear);
      });
      keep(region);
      if (region) regions.push(region);
    }
    paintSheet(sheet, regions, color);
  });
};
