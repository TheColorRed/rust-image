import { Color } from '@abra/core/color';
import { gaussianBlur } from '@abra/core/filters';
import { Area, rectOrigin } from '@abra/core/geometry';
import { Falloff, Mask, type MaskLike } from '@abra/core/mask';
import { glowGradient, shadeGradient } from '@/src/lib/ui/color-bars';
import { type ToolContext } from '@/src/lib/project/edit-sections';
import { derivedFor } from '@/src/lib/editor/editor-preloads';
import { belongsTo } from '@/src/lib/ai/faces';
import { owning } from '@/src/lib/native/native-handles';
import { paintSheet, type Painting } from '@/src/lib/beauty/layers';

type Sculpt = 'highlighter' | 'contour';

/** How much of the way from a stroke's middle to its edge the fade takes up, at the hardest and the softest: the Softness slider's 0 and 1. */
const SHORTEST_FADE = 0.4;
const LONGEST_FADE = 1;
/** How steeply a cheek stroke falls going in towards the mouth: this much down for every width across. */
const TILT = 0.45;

/** A stroke: a soft oval at a place, turned by `angle` (radians), with half length and half thickness. */
type Stroke = { x: number; y: number; length: number; thickness: number; angle: number };

/**
 * Paints the highlighter (a light glow) or the contour (a shadow) on the cheekbones and the nose of the person in `context`, or of
 * everyone when it names none, on `sheet`, which becomes a layer over the photo, with an edge from hard at `softness` 0 to soft at
 * 1, as big as `size` (1 is as made); the contour is as dark as the `shade` (0 to 1 along the bar of shades) says. The highlighter goes
 * on the top of the cheekbone, down the bridge of the nose, and on the middle of the forehead and the chin; the contour goes in the
 * hollow under the cheekbone and down both sides of the nose. How much of it shows is the layer's opacity and how it mixes with the
 * skin is the layer's blend, so the light and shade of the skin show through, and it stays on skin.
 */
export const applySculpt = (
  kind: Sculpt,
  sheet: Painting,
  softness: number,
  size: number,
  shade: number,
  { preloaded, person, others }: ToolContext,
  fan = 0,
) => {
  if (!preloaded || Array.isArray(preloaded)) return;
  owning(keep => {
    // Both are the shade picked on a bar: the highlighter's goes from a pearl white through champagne and gold to rose, and the
    // contour's from a tan, a light shadow, to a dark brown, a deep one.
    const color = keep((kind === 'highlighter' ? glowGradient() : shadeGradient()).colorAt(shade));
    const regions: MaskLike[] = [];
    for (const key of Object.keys(preloaded)) {
      const [maskKind, box, side] = key.split('|');
      if (maskKind !== 'eye' || !box || !side) continue;
      if (!belongsTo(box, sheet, person, others)) continue;
      const region = derivedFor(preloaded, `${kind}|${key}`, `${softness.toFixed(3)}|${size.toFixed(3)}|${fan.toFixed(3)}`, () =>
        owning(own => {
          const bounds = (preloaded[key] as MaskLike).bounds(0);
          if (!bounds) return undefined;
          const other = (preloaded[`eye|${box}|${side === 'left' ? 'right' : 'left'}`] as MaskLike | undefined)?.bounds(0);
          const centerX = bounds.x + bounds.width / 2;
          const midline = other ? (centerX + other.x + other.width / 2) / 2 : sheet.width() / 2;
          const outwards = centerX < midline ? -1 : 1;
          const eyeWidth = bounds.width;
          const eyeBottom = bounds.y + bounds.height;
          const lips = (preloaded[`lipstick|${box}`] as MaskLike | undefined)?.bounds(0);
          const drop = (lips ? lips.y + lips.height / 2 : eyeBottom + eyeWidth * 2.5) - eyeBottom;
          // The cheek runs from high on the outside down towards the mouth on the inside.
          const angle = Math.atan(-outwards * TILT);

          const strokes: Stroke[] = [];
          let trapezoid: number[][] | undefined;
          // The cheek's stroke, and the way along it that goes in towards the nose and mouth.
          let cheek: Stroke;
          let inward = Math.atan2(-outwards * Math.sin(angle), -outwards * Math.cos(angle));
          if (kind === 'highlighter') {
            cheek = {
              x: centerX + outwards * eyeWidth * 0.7,
              y: eyeBottom + Math.max(drop * 0.18, eyeWidth * 0.3),
              length: eyeWidth * 1.0 * size,
              thickness: eyeWidth * 0.4 * size,
              angle,
            };
          } else {
            // The hollow runs from the side of the face, level with the nostrils, in towards the corner of the mouth, stopping short of
            // it. It is placed from the edge of the face the parser found and the corner of the mouth, not from the eye, so it
            // follows a face that is turned to one side.
            const face = (preloaded[`skin|${box}`] as MaskLike | undefined)?.bounds(0);
            if (face && lips) {
              const outerX = face.x + face.width * (outwards === -1 ? 0.06 : 0.94);
              const outerY = eyeBottom + drop * 0.45;
              const innerX = outwards === -1 ? lips.x : lips.x + lips.width;
              const innerY = lips.y + lips.height * 0.4;
              const run = Math.hypot(innerX - outerX, innerY - outerY);
              inward = Math.atan2(innerY - outerY, innerX - outerX);
              cheek = {
                x: outerX + (innerX - outerX) * 0.4,
                y: outerY + (innerY - outerY) * 0.4,
                length: run * 0.38 * size,
                thickness: run * 0.2 * size,
                angle: inward,
              };
            } else {
              cheek = {
                x: centerX + outwards * eyeWidth * 1.15,
                y: eyeBottom + drop * 0.55,
                length: eyeWidth * 1.9 * size,
                thickness: eyeWidth * 0.55 * size,
                angle,
              };
            }
          }
          // Without a fan the cheek is the soft oval. With one it is a single trapezoid from the edge of the face towards the nose or
          // mouth: as thick as the oval's tip at the outer end, whatever the fan, and up to three times as thick as the stroke at the
          // inner end. One solid shape, so there is no step where an oval would meet it.
          if (fan > 0) {
            const inwardX = Math.cos(inward);
            const inwardY = Math.sin(inward);
            const normalX = -inwardY;
            const normalY = inwardX;
            const outerWidth = cheek.thickness * 0.5;
            const innerWidth = cheek.thickness * (1 + fan * 2);
            const outerX = cheek.x - inwardX * cheek.length;
            const outerY = cheek.y - inwardY * cheek.length;
            const innerX = cheek.x + inwardX * cheek.length;
            const innerY = cheek.y + inwardY * cheek.length;
            trapezoid = [
              [outerX + normalX * outerWidth, outerY + normalY * outerWidth],
              [innerX + normalX * innerWidth, innerY + normalY * innerWidth],
              [innerX - normalX * innerWidth, innerY - normalY * innerWidth],
              [outerX - normalX * outerWidth, outerY - normalY * outerWidth],
            ];
          } else {
            strokes.push(cheek);
          }
          // What runs down the middle of the face is drawn once for the person, with the eye on the left of the photo (or the only
          // eye there is).
          const nose = (preloaded[`nose|${box}`] as MaskLike | undefined)?.bounds(0);
          if (nose && (outwards === -1 || !other)) {
            const noseX = nose.x + nose.width / 2;
            const noseY = nose.y + nose.height * 0.45;
            const length = nose.height * 0.4 * size;
            if (kind === 'highlighter') {
              // Down the bridge of the nose.
              strokes.push({ x: noseX, y: noseY, length, thickness: nose.width * 0.22 * size, angle: Math.PI / 2 });
              // The middle of the forehead, above the brows.
              const brow = (preloaded[`brow|${box}|${side}`] as MaskLike | undefined)?.bounds(0);
              if (brow) {
                strokes.push({
                  x: noseX,
                  y: brow.y - eyeWidth * 0.8,
                  length: eyeWidth * 0.9 * size,
                  thickness: eyeWidth * 0.5 * size,
                  angle: 0,
                });
              }
              // The chin, under the lower lip.
              if (lips) {
                strokes.push({
                  x: noseX,
                  y: lips.y + lips.height + lips.height * 0.9,
                  length: eyeWidth * 0.6 * size,
                  thickness: lips.height * 0.45 * size,
                  angle: 0,
                });
              }
            } else {
              // Broad and soft down both sides of the nose, so the nose looks narrower, not outlined.
              for (const edge of [-1, 1]) {
                strokes.push({
                  x: noseX + edge * nose.width * 0.58,
                  y: noseY + nose.height * 0.15,
                  length: nose.height * 0.3 * size,
                  thickness: nose.width * 0.22 * size,
                  angle: Math.PI / 2,
                });
              }
            }
          }

          // Made on a crop around the strokes and put back on the photo.
          const reach = (stroke: Stroke) => Math.max(stroke.length, stroke.thickness) * 1.1;
          // The trapezoid's edge is softened by a blur, so the crop leaves room for it.
          const trapezoidFeather = eyeWidth * (0.1 + 0.3 * softness);
          const margin = trapezoidFeather * 3;
          const trapezoidPoints = trapezoid ?? [];
          const left = Math.max(0, Math.floor(Math.min(...strokes.map(stroke => stroke.x - reach(stroke)), ...trapezoidPoints.map(point => point[0] - margin))));
          const top = Math.max(0, Math.floor(Math.min(...strokes.map(stroke => stroke.y - reach(stroke)), ...trapezoidPoints.map(point => point[1] - margin))));
          const right = Math.min(sheet.width(), Math.ceil(Math.max(...strokes.map(stroke => stroke.x + reach(stroke)), ...trapezoidPoints.map(point => point[0] + margin))));
          const bottom = Math.min(sheet.height(), Math.ceil(Math.max(...strokes.map(stroke => stroke.y + reach(stroke)), ...trapezoidPoints.map(point => point[1] + margin))));
          const width = right - left;
          const height = bottom - top;
          if (width < 2 || height < 2) return undefined;
          const fade = SHORTEST_FADE + (LONGEST_FADE - SHORTEST_FADE) * softness;
          // Each stroke is a soft oval on the crop, made by the library.
          const blobsStarted = Date.now();
          let shape = own(
            Mask.fromBlobs(
              width,
              height,
              strokes.map(stroke => ({
                center: { x: stroke.x - left, y: stroke.y - top },
                halfSize: { width: stroke.length, height: stroke.thickness },
                angle: stroke.angle,
                falloff: Falloff.Smooth.new({ fade }),
              })),
            ),
          );
          if (__DEV__) console.debug(`[blobs] ${kind} ${width}x${height}, ${strokes.length} strokes ${(Date.now() - blobsStarted).toFixed(0)} ms`);
          if (trapezoid) {
            const area = own(Area.fromPoints(trapezoid.map(point => [point[0] - left, point[1] - top])));
            const trapezoidMask = own(Mask.empty(width, height));
            trapezoidMask.drawArea(area, own(Color.fromRgba(255, 255, 255, 255)), rectOrigin(area.bounds()));
            const trapezoidImage = own(trapezoidMask.toImage());
            own(gaussianBlur(Math.max(1.5, trapezoidFeather))).apply(trapezoidImage);
            shape = own(shape.maximum(own(Mask.fromImage(trapezoidImage))));
          }
          // Only on skin or nose, with a soft edge so it does not stop at a line.
          const skin = preloaded[`skin|${box}`] as MaskLike | undefined;
          if (skin) {
            const crop = { x: left, y: top, width, height };
            const noseMask = preloaded[`nose|${box}`] as MaskLike | undefined;
            const skinHere = own(skin.cropped(crop));
            const allowed = noseMask ? own(skinHere.maximum(own(noseMask.cropped(crop)))) : skinHere;
            const allowedImage = own(allowed.toImage());
            own(gaussianBlur(Math.max(1.5, eyeWidth * 0.12))).apply(allowedImage);
            shape = own(shape.minimum(own(Mask.fromImage(allowedImage))));
          }
          return shape.placed(sheet.width(), sheet.height(), { x: left, y: top });
        }),
      ) as MaskLike | undefined;
      if (region) regions.push(region);
    }
    paintSheet(sheet, regions, color);
  });
};
