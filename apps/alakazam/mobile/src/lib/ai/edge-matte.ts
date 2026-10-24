import { devDebug } from '@/src/lib/log';
import { ResizeTarget } from '@abra/core/canvas';
import { type ImageLike } from '@abra/core/image';
import { Mask, TransformAlgorithm, type MaskLike } from '@abra/core/mask';
import { resize } from '@abra/core/transform';
import { runAiModelGuidedAsync } from '@alakazam/mobile';
import { type model } from '@/src/lib/ai/faces';

/**
 * How far from the edge of the rough mask, as a fraction of the photo's width, the edge refinement model is left to decide: the trimap is
 * sure subject inside this, sure background outside it, and unknown in the band between. Wide enough for the wisps the first model missed.
 */
export const MATTE_UNKNOWN = 0.012;

/**
 * How the edge refinement model is given the rough mask. The photo the masks are made on is small (a preview), so a strand of hair is a pixel or
 * two across and the model, which sees 1024 pixels at most, has little to go on. The part of the photo with the mask is enlarged by
 * `MATTE_SCALE` and given to the model in tiles of `MATTE_TILE` pixels that overlap by `MATTE_OVERLAP`, each tile with the model's full attention.
 */
const MATTE_SCALE = 1.5;
const MATTE_TILE = 1024;
const MATTE_OVERLAP = 128;

/** Where tiles start along a side of `length` pixels: the first at 0, the last ending at the side, and each overlapping the one before. */
const tileStarts = (length: number): number[] => {
  if (length <= MATTE_TILE) return [0];
  const count = Math.ceil((length - MATTE_OVERLAP) / (MATTE_TILE - MATTE_OVERLAP));
  return Array.from({ length: count }, (_, at) =>
    at === count - 1 ? length - MATTE_TILE : at * (MATTE_TILE - MATTE_OVERLAP),
  );
};

/**
 * The alpha of what a rough mask found, from the edge refinement model, as a mask of the whole photo: `mask` is the trimap's guess and
 * the model decides the band around its edge. Only the part of the photo the band is in is looked at, enlarged and in tiles (see `MATTE_SCALE`).
 * Nothing when the mask is empty.
 */
export const edgeMatte = async (
  matting: NonNullable<ReturnType<typeof model>>,
  photo: ImageLike,
  mask: MaskLike,
  keep: <V>(value: V) => V,
  progress: (share: number) => void,
): Promise<MaskLike | undefined> => {
  const reach = Math.max(2, Math.round(photo.width() * MATTE_UNKNOWN));
  const grown = keep(mask.dilated(reach));
  const box = grown.bounds(0);
  if (!box) return undefined;
  const [x, y] = [Math.max(0, Math.floor(box.x)), Math.max(0, Math.floor(box.y))];
  const part = {
    x,
    y,
    width: Math.min(photo.width(), Math.ceil(box.x + box.width)) - x,
    height: Math.min(photo.height(), Math.ceil(box.y + box.height)) - y,
  };
  if (part.width < 1 || part.height < 1) return undefined;
  const unknown = keep(grown.toImage());
  unknown.mul(128 / 255);
  const trimap = keep(
    keep(Mask.fromImage(unknown))
      .maximum(keep(mask.eroded(reach)))
      .cropped(part),
  );

  // The part of the photo and its trimap, enlarged.
  const [width, height] = [Math.round(part.width * MATTE_SCALE), Math.round(part.height * MATTE_SCALE)];
  const picture = keep(photo.cropped(part.x, part.y, part.width, part.height));
  keep(resize(ResizeTarget.Exact.new({ width, height }))).apply(picture);
  const guide = keep(trimap.resized(width, height, TransformAlgorithm.Bilinear));

  // Each tile is run alone and only its middle is kept, where the model has context all around: a tile loses its outer half-overlap on each
  // side that has a neighbor. A tile with nothing for the model to decide is skipped.
  const xs = tileStarts(width);
  const ys = tileStarts(height);
  const trim = MATTE_OVERLAP / 2;
  let alpha: MaskLike | undefined;
  for (const [row, top] of ys.entries()) {
    for (const [column, left] of xs.entries()) {
      const tile = { x: left, y: top, width: Math.min(MATTE_TILE, width), height: Math.min(MATTE_TILE, height) };
      const tileGuide = keep(guide.cropped(tile));
      const done = () => progress((row * xs.length + column + 1) / (xs.length * ys.length));
      if (!tileGuide.bounds(0)) {
        done();
        continue;
      }
      const found = keep(
        await runAiModelGuidedAsync(
          matting,
          keep(picture.cropped(tile.x, tile.y, tile.width, tile.height)) as Parameters<typeof runAiModelGuidedAsync>[1],
          tileGuide as Parameters<typeof runAiModelGuidedAsync>[2],
        ),
      );
      const [cutLeft, cutTop] = [column > 0 ? trim : 0, row > 0 ? trim : 0];
      const [cutRight, cutBottom] = [column < xs.length - 1 ? trim : 0, row < ys.length - 1 ? trim : 0];
      const middle = keep(
        keep(found.mask()).cropped({
          x: cutLeft,
          y: cutTop,
          width: tile.width - cutLeft - cutRight,
          height: tile.height - cutTop - cutBottom,
        }),
      );
      const placed = keep(middle.placed(width, height, { x: left + cutLeft, y: top + cutTop }));
      alpha = alpha ? keep(alpha.maximum(placed)) : placed;
      done();
    }
  }
  devDebug('matte', 'edge refinement looked at', width, 'x', height, 'in', xs.length * ys.length, 'tiles');
  if (!alpha) return undefined;
  return keep(alpha.resized(part.width, part.height, TransformAlgorithm.Bilinear)).placed(
    photo.width(),
    photo.height(),
    {
      x: part.x,
      y: part.y,
    },
  );
};
