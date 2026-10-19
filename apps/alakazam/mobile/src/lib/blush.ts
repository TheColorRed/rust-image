import { Color } from '@abra/core/color';
import { gaussianBlur } from '@abra/core/filters';
import { Falloff, Mask, type MaskLike } from '@abra/core/mask';
import { type ToolContext } from '@/src/lib/edit-sections';
import { derivedFor } from '@/src/lib/editor/editor-preloads';
import { belongsTo } from '@/src/lib/faces';
import { owning } from '@/src/lib/native-handles';
import { paintSheet, type Painting } from '@/src/lib/layers';

/** How far down from the eye to the middle of the mouth the middle of the blush sits: the apple of the cheek. */
const DOWN_TO_MOUTH = 0.4;
/** How far out from the eye's middle (away from the middle of the face), in eye widths, the blush's middle is moved. */
const OUTWARDS = 0.25;
/** The blush's half width and half height, in eye widths, at the Size slider's 1. */
const HALF_WIDTH = 1.0;
const HALF_HEIGHT = 0.7;
/** How quickly the blush fades towards its edge at the hardest and the softest: the Softness slider's 0 and 1. */
const SHARPEST = 6;
const SOFTEST = 1.5;

/**
 * Paints blush of a hue (0 to 1 along `Gradient.hue()`) on the cheeks of the person in `context`, or of everyone when it names none,
 * on `sheet`, which becomes a layer over the photo, with an edge from hard at `softness` 0 to soft at 1, as wide as `size` (1 is
 * as made), as colorful as `saturation` and as bright as `brightness` (both 0 to 1). How much of it shows is the layer's opacity and
 * how it mixes with the skin is the layer's blend, so the light and shade of the skin show through. The blush stays on skin: never
 * over the eye, the nose or the lips.
 */
export const applyBlush = (
  sheet: Painting,
  hue: number,
  softness: number,
  size: number,
  saturation: number,
  brightness: number,
  { preloaded, person, others }: ToolContext,
) => {
  if (!preloaded || Array.isArray(preloaded)) return;
  owning(keep => {
    const color = keep(Color.fromHsv(360 * (1 - hue), saturation, brightness));
    const regions: MaskLike[] = [];
    for (const key of Object.keys(preloaded)) {
      const [kind, box, side] = key.split('|');
      if (kind !== 'eye' || !box || !side) continue;
      if (!belongsTo(box, sheet, person, others)) continue;
      const region = derivedFor(preloaded, `blush|${key}`, `${softness.toFixed(3)}|${size.toFixed(3)}`, () =>
        owning(own => {
          const eye = preloaded[key] as MaskLike;
          const bounds = eye.bounds(0);
          if (!bounds) return undefined;
          const other = (preloaded[`eye|${box}|${side === 'left' ? 'right' : 'left'}`] as MaskLike | undefined)?.bounds(0);
          const centerX = bounds.x + bounds.width / 2;
          // Away from the other eye, or from the middle of the photo when there is only one.
          const midline = other ? (centerX + other.x + other.width / 2) / 2 : sheet.width() / 2;
          const outwards = centerX < midline ? -1 : 1;
          const lips = (preloaded[`lipstick|${box}`] as MaskLike | undefined)?.bounds(0);
          const eyeBottom = bounds.y + bounds.height;
          const mouthY = lips ? lips.y + lips.height / 2 : eyeBottom + bounds.width * 2.5;
          const middleX = centerX + outwards * bounds.width * OUTWARDS;
          const middleY = eyeBottom + (mouthY - eyeBottom) * DOWN_TO_MOUTH;
          if (__DEV__) {
            console.debug(
              `[blush] face ${box} ${side} eye ${JSON.stringify(bounds)} lips ${JSON.stringify(lips)} middle ${Math.round(middleX)},${Math.round(middleY)} in ${sheet.width()}x${sheet.height()}, person ${JSON.stringify(person)}`,
            );
          }
          const halfWidth = bounds.width * HALF_WIDTH * size;
          const halfHeight = bounds.width * HALF_HEIGHT * size;

          // The blush is made on a crop around it and put back on the photo.
          const left = Math.max(0, Math.floor(middleX - halfWidth * 1.3));
          const top = Math.max(0, Math.floor(middleY - halfHeight * 1.3));
          const right = Math.min(sheet.width(), Math.ceil(middleX + halfWidth * 1.3));
          const bottom = Math.min(sheet.height(), Math.ceil(middleY + halfHeight * 1.3));
          const width = right - left;
          const height = bottom - top;
          if (width < 2 || height < 2) return undefined;
          const fade = SHARPEST + (SOFTEST - SHARPEST) * softness;
          // A soft oval on the crop, made by the library.
          const blobsStarted = Date.now();
          let shape = own(
            Mask.fromBlobs(width, height, [
              {
                center: { x: middleX - left, y: middleY - top },
                halfSize: { width: halfWidth, height: halfHeight },
                angle: 0,
                falloff: Falloff.Gaussian.new({ fade, reach: 1.7 }),
              },
            ]),
          );
          if (__DEV__) console.debug(`[blobs] blush ${width}x${height} oval ${(Date.now() - blobsStarted).toFixed(0)} ms`);
          // Only on skin, with a soft edge so it does not stop at a line.
          const skin = preloaded[`skin|${box}`] as MaskLike | undefined;
          if (skin) {
            const skinImage = own(own(skin.cropped({ x: left, y: top, width, height })).toImage());
            own(gaussianBlur(Math.max(1.5, bounds.width * 0.15))).apply(skinImage);
            shape = own(shape.minimum(own(Mask.fromImage(skinImage))));
          }
          return shape.placed(sheet.width(), sheet.height(), { x: left, y: top });
        }),
      ) as MaskLike | undefined;
      if (region) regions.push(region);
    }
    paintSheet(sheet, regions, color);
  });
};
