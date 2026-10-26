import { derivedFor, belongsTo, owning, type Painting, type ToolContext } from '@alakazam/plugin';
import { brightness, saturation } from '@abra/core/adjustments';
import { teethMask } from '@abra/core/beauty';
import { type MaskLike } from '@abra/core/mask';

/** How far past the mouth the photo is cropped, as a fraction of the mouth's size, so the mask's soft edge is not cut off. */
const MARGIN = 0.15;

/**
 * Whitens and brightens the teeth of the person in `context`, or of everyone when it names none, by `whiten` and `brighten` (each 0
 * to 100): on `sheet`, which becomes a layer over the photo, the part of the photo around the mouths with its color taken out as much
 * as `whiten` and made lighter as much as `brighten`, shown only where the library finds teeth (bright, yellowish-white pixels
 * inside the mouth), so the gums and tongue keep their color.
 */
export const applyTeethWhitening = (
  sheet: Painting,
  whiten: number,
  brighten: number,
  { preloaded, person, others }: ToolContext,
) => {
  if (!preloaded || Array.isArray(preloaded) || (whiten <= 0 && brighten <= 0)) return;
  // Each face is searched in its own mouth, or in its lips when the face analysis found no inside-of-mouth area (a smile often has
  // none), where the teeth color test skips the pink lips.
  const faces: Record<string, { mouth?: MaskLike; lips?: MaskLike }> = {};
  for (const [key, mask] of Object.entries(preloaded)) {
    const [kind, box] = key.split('|');
    if (!box || !belongsTo(box, sheet, person, others)) continue;
    faces[box] ??= {};
    if (kind === 'mouth') faces[box].mouth = mask as MaskLike;
    if (kind === 'lipstick') faces[box].lips = mask as MaskLike;
  }
  const mouths = Object.values(faces).flatMap(face => face.mouth ?? face.lips ?? []);
  if (mouths.length === 0) return;
  // One face with only lips is searched all through its small crop. With more faces the crop reaches between them, so the search
  // stays inside their mouths.
  const limited = mouths.length > 1 || Object.values(faces).some(face => face.mouth);
  owning(keep => {
    const reach = mouths.slice(1).reduce((all, mouth) => keep(all.maximum(mouth)), mouths[0]);
    const bounds = reach.bounds(0);
    if (!bounds) return;
    const x = Math.max(0, Math.floor(bounds.x - bounds.width * MARGIN));
    const y = Math.max(0, Math.floor(bounds.y - bounds.height * MARGIN));
    const part = {
      x,
      y,
      width: Math.min(sheet.width(), Math.ceil(bounds.x + bounds.width * (1 + MARGIN))) - x,
      height: Math.min(sheet.height(), Math.ceil(bounds.y + bounds.height * (1 + MARGIN))) - y,
    };
    if (part.width < 1 || part.height < 1) return;
    const image = sheet.photo.cropped(part.x, part.y, part.width, part.height);
    // The teeth are found by their color, which the amount does not change, so they are found once for these mouths and kept while
    // the amount moves. The editor owns the mask and frees it with the preloads.
    const teeth = derivedFor(preloaded, 'teeth', `${limited}|${JSON.stringify(part)}|${mouths.length}`, () =>
      teethMask(image, limited ? keep(reach.cropped(part)) : undefined, 1),
    ) as MaskLike;
    // Whiten takes the color out of the teeth, and brighten makes them lighter, up to 40 at the top of the slider. Neither changes
    // anything at 0, so the layer can cover the teeth at full strength.
    keep(saturation(-whiten)).apply(image);
    keep(brightness(Math.min(brighten, 100) * 0.4)).apply(image);
    teeth.applyToImage(image);
    sheet.painted = { image, x: part.x, y: part.y };
  });
};


