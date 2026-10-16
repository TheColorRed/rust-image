import { exposure } from '@abra/core/adjustments';
import { blend, BlendMode } from '@abra/core/blend';
import { Color } from '@abra/core/color';
import { guidedFilter } from '@abra/core/filters';
import { Image } from '@abra/core/image';
import { TransformAlgorithm, type MaskLike } from '@abra/core/mask';
import { type ToolContext } from '@/src/lib/edit-sections';
import { derivedFor } from '@/src/lib/editor/editor-preloads';
import { owning } from '@/src/lib/faces';
import { people } from '@/src/state/analysis';
import { type Painting } from '@/src/lib/layers';

/**
 * How the hair's edge is made to follow the photo (`guidedFilter`). The hair mask comes from a model that looked at a small copy of the
 * photo, so its edge is soft and can sit a few pixels off. It looks this far around each pixel, as a fraction of the photo's width, and
 * follows an edge of the photo that is at least this strong (colors counted from 0 to 1): a smaller number follows fainter edges.
 */
const EDGE_REACH = 0.004;
const EDGE_STRENGTH = 0.0005;
/** The most Lighten / Darken changes the hair, in stops of light, and the most it lifts the darkest hair when lightening. */
const MOST_STOPS = 4;
const MOST_LIFT = 0.15;

/**
 * Colors the hair of the person in `context`, or of everyone when it names none, on `sheet`, which becomes a layer over the photo.
 * The hair's own light and shade are kept: the color (a hue 0 to 1 along `Gradient.hue()`, as colorful as `saturation`) is laid with
 * the Color blend, and then `shift` (-1 darkest to 1 lightest) makes the hair darker or lighter, so black hair and blond hair are
 * both a color and a shade. How much of it shows is the layer's opacity.
 */
export const applyHairColor = (
  sheet: Painting,
  hue: number,
  saturation: number,
  shift: number,
  { preloaded, person, others }: ToolContext,
) => {
  if (!preloaded || Array.isArray(preloaded)) return;
  owning(keep => {
    const hair = preloaded['hair|all'] as MaskLike | undefined;
    if (!hair) return;
    // The hair that is wanted and its edge are made once, and kept while the sliders move: only a different person (or different people
    // with their own value) changes them. The hair is of the whole photo, so one person's is the hair on their body, and everyone's is
    // the hair on any body, leaving out the people who have their own. Their bodies are found at their own size, and the hair is at the
    // size the models looked at. The edge is then made to follow the photo.
    const kept = `${person?.id ?? ''}|${(others ?? []).join(',')}`;
    const region = derivedFor(preloaded, 'hair|refined', kept, () =>
      owning(own => {
        const bodies = (ids: number[]) =>
          ids.flatMap(id => {
            const body = people.value?.find(found => found.id === id)?.mask;
            return body ? [own(body.resized(hair.width(), hair.height(), TransformAlgorithm.Bilinear))] : [];
          });
        let wanted = hair;
        if (person?.id !== undefined) {
          for (const body of bodies([person.id])) wanted = own(wanted.multiplied(body));
        } else {
          // With no people found there is nothing to keep the hair on, so it stands as the model found it.
          const everyone = bodies((people.value ?? []).map(found => found.id)).reduce<MaskLike | undefined>(
            (all, body) => (all ? own(all.maximum(body)) : body),
            undefined,
          );
          if (everyone) wanted = own(wanted.multiplied(everyone));
          for (const body of bodies(others ?? [])) wanted = own(wanted.subtracted(body));
        }
        const reach = wanted.bounds(0);
        if (!reach) return undefined;
        // Only the part of the photo the hair is in is looked at.
        const [x, y] = [Math.max(0, Math.floor(reach.x)), Math.max(0, Math.floor(reach.y))];
        const part = {
          x,
          y,
          width: Math.min(sheet.width(), Math.ceil(reach.x + reach.width)) - x,
          height: Math.min(sheet.height(), Math.ceil(reach.y + reach.height)) - y,
        };
        if (part.width < 1 || part.height < 1) return undefined;
        const crop = own(sheet.photo.cropped(part.x, part.y, part.width, part.height));
        const edge = own(guidedFilter(own(wanted.cropped(part)), crop, Math.max(1, Math.round(sheet.width() * EDGE_REACH)), EDGE_STRENGTH));
        return edge.placed(sheet.width(), sheet.height(), { x, y });
      }),
    ) as MaskLike | undefined;
    if (!region) return;
    const bounds = region.bounds(0);
    if (!bounds) return;
    const [x, y] = [Math.floor(bounds.x), Math.floor(bounds.y)];
    const part = {
      x,
      y,
      width: Math.min(sheet.width(), Math.ceil(bounds.x + bounds.width)) - x,
      height: Math.min(sheet.height(), Math.ceil(bounds.y + bounds.height)) - y,
    };
    if (part.width < 1 || part.height < 1) return;
    const image = sheet.photo.cropped(part.x, part.y, part.width, part.height);
    const color = keep(Color.fromHsv(360 * (1 - hue), saturation, 0.8));
    const paint = keep(Image.newFromColor(part.width, part.height, color));
    keep(keep(blend(paint)).withMode(BlendMode.Color)).apply(image);
    if (shift !== 0) {
      // Darker is fewer stops of light; lighter is more, and also lifts the shadows, since hair that is nearly black has no light to
      // multiply and would otherwise stay black.
      const lighter = Math.max(0, shift);
      keep(keep(exposure(shift * MOST_STOPS)).withOffset(lighter * MOST_LIFT)).apply(image);
    }
    keep(region.cropped(part)).applyToImage(image);
    sheet.painted = { image, x, y };
  });
};
