import { channelMix } from '@abra/core/adjustments';
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
/**
 * How light the hair ends up. Lighten / Darken (-1 to 1) is where the hair's average brightness (0 to 1) is moved to: `MIDDLE` at 0, and
 * `REACH` either way, so -1 is nearly black and 1 nearly white. It is where the hair ends up and not how far it moves, so dark brown
 * hair can become blonde as well as blonde hair can become black.
 */
const MIDDLE = 0.5;
const REACH = 0.45;
/** The brightness (0 to 1) that the strands of light hair vary by. Dark hair has less in a photo, so moving it to light hair as it is would look flat. */
const NATURAL_SPREAD = 0.13;
/** The most the hair's variation is stretched by when it is made lighter. */
const MOST_STRETCH = 4;
/** How far, in spreads, the hair's lightest and darkest strands are kept from white and black, so they are not cut off. */
const HEADROOM = 2.5;

/** The average brightness and the spread of it (how much the strands vary, as a standard deviation), both 0 to 1, of the hair under a mask. */
type Tones = { mean: number; spread: number };
/** What `tonesOf` found for each hair, which only changes with the hair, kept while the sliders move. */
const toneCache = new WeakMap<object, Tones>();

/** The brightness of the pixels of `image`, weighted by `weights` (the hair mask over it), as the Color blend's luma. */
const tonesOf = (hair: MaskLike, image: { rgba(): ArrayBuffer }, weights: ArrayBuffer): Tones => {
  const known = toneCache.get(hair);
  if (known) return known;
  const pixels = new Uint8Array(image.rgba());
  const weight = new Uint8Array(weights);
  let sum = 0;
  let squares = 0;
  let total = 0;
  for (let at = 0; at < weight.length; at++) {
    if (!weight[at]) continue;
    const light = 0.299 * pixels[at * 4] + 0.587 * pixels[at * 4 + 1] + 0.114 * pixels[at * 4 + 2];
    sum += weight[at] * light;
    squares += weight[at] * light * light;
    total += weight[at];
  }
  const mean = total ? sum / total : 128;
  const found = { mean: mean / 255, spread: Math.sqrt(Math.max(0, (total ? squares / total : 0) - mean * mean)) / 255 };
  toneCache.set(hair, found);
  return found;
};

/**
 * Colors the hair of the person in `context`, or of everyone when it names none, on `sheet`, which becomes a layer over the photo.
 * The hair's own light and shade are kept. First its brightness is moved to where `shift` (-1 darkest to 1 lightest) says, keeping the
 * variation of the strands (stretched when light hair is made from dark, so it does not look flat), and then the color (a hue 0 to 1
 * along `Gradient.hue()`, as colorful as `saturation`) is laid with the Color blend, which keeps that brightness. That order is what lets
 * dark brown hair become blonde: laying the color first leaves it dark. How much of it shows is the layer's opacity.
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
    const here = keep(region.cropped(part));
    // Moves the hair's average brightness to the target: each pixel becomes `gain * pixel + lift`, which is the strands' variation times
    // `gain` around the new average. The gain stretches the variation of dark hair that is being lightened (up to what light hair has),
    // and is held back so the lightest and darkest strands are not cut off, and so the lift is never negative (the gain applies first and
    // cannot take a pixel below 0).
    const { mean, spread } = tonesOf(region, image, here.values());
    const target = Math.min(1, Math.max(0, MIDDLE + shift * REACH));
    const wide = Math.max(spread, 0.01);
    const stretch = target >= mean ? Math.min(MOST_STRETCH, Math.max(1, NATURAL_SPREAD / wide)) : 1;
    const gain = Math.max(0.2, Math.min(stretch, Math.min(target, 1 - target) / (HEADROOM * wide), target / Math.max(mean, 0.01)));
    const lift = Math.max(0, target - gain * mean);
    if (Math.abs(gain - 1) > 0.01) keep(channelMix([gain, 0, 0], [0, gain, 0], [0, 0, gain])).apply(image);
    if (lift > 0.004) {
      const light = keep(Image.newFromColor(part.width, part.height, keep(Color.fromHsv(0, 0, lift))));
      keep(keep(blend(light)).withMode(BlendMode.LinearDodge)).apply(image);
    }
    const color = keep(Color.fromHsv(360 * (1 - hue), saturation, 0.8));
    const paint = keep(Image.newFromColor(part.width, part.height, color));
    keep(keep(blend(paint)).withMode(BlendMode.Color)).apply(image);
    here.applyToImage(image);
    sheet.painted = { image, x, y };
  });
};
