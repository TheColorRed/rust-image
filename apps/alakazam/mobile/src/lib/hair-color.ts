import { channelMix, gradientMap, grayscale, invert } from '@abra/core/adjustments';
import { blend, BlendMode } from '@abra/core/blend';
import { Color } from '@abra/core/color';
import { gaussianBlur, guidedFilter } from '@abra/core/filters';
import { ColorStop, Gradient } from '@abra/core/gradient';
import { Image, type ImageLike } from '@abra/core/image';
import { Mask, TransformAlgorithm, type MaskLike } from '@abra/core/mask';
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
/** The mask value (0 to 255) below which the refined edge is cut off, so the faint fringe of it does not tint what is around the hair. */
const EDGE_CUT_SOFT = 20;
const EDGE_CUT_PRECISE = 80;
/** How much firmer (times) the edge is made at its most precise, for light colors, and the brightness (0 to 1) the colors go from soft to precise over. */
const EDGE_GAIN_PRECISE = 2;
const EDGE_LIGHT_FROM = 0.3;
const EDGE_LIGHT_SPAN = 0.5;
/** How far the hair mask is grown before its edge is fitted to the photo, as a fraction of the photo's width, and how much the skin the model found counts for (times). */
const EDGE_GROW = 0.0025;
const SKIN_STRENGTH = 1.5;
/** How much the model's hair mask is strengthened before its edge is made to follow the photo, so thin wisps it was unsure of are colored. */
const WISP_BOOST = 2;
/**
 * How the pixels along the edge are decided by color (see `applyHairColor`), as fractions of the photo's width: how wide the band of
 * uncertain pixels is on each side of the fitted edge, and how far the hair and background colors are spread outwards to reach under it.
 * `COLOR_MIX_LEAST` is how far apart the hair and background colors must be (0 to 1) for a pixel's color to decide fully.
 */
const BAND_REACH = 0.005;
const BAND_SPREAD = 0.02;
const COLOR_MIX_LEAST = 0.12;
/** How much the skin the model found counts for (times) in the last cut that keeps the hair off skin, which is stronger than the first (`SKIN_STRENGTH`). */
const SKIN_AT_EDGE = 3;
/** Where the hair's own color near each pixel started, for cutting it down to the part being colored: it is a picture of the part of the photo the hair is in. */
const foregroundOrigin = new WeakMap<object, { x: number; y: number }>();

/**
 * How light the hair's middle color is. Lighten / Darken (-1 to 1) is the brightness (HSV value, 0 to 1) of the color the hair's average
 * tone becomes: `HAIR_MIDDLE` at 0, and `HAIR_REACH` either way, so -1 is nearly black and 1 nearly white. It is where the hair ends up
 * and not how far it moves, so dark brown hair can become blonde as well as blonde hair can become black.
 */
export const HAIR_MIDDLE = 0.5;
export const HAIR_REACH = 0.45;
/**
 * The gradient the hair's tones are mapped to is made from the one color: its shadows are darker and a little more colorful (the roots, and
 * the hair between strands), its highlights are lighter and paler (the sheen). How much darker, lighter and paler, as fractions of the way
 * to black, to white, and to gray.
 */
const SHADOW_DARKER = 0.7;
const SHADOW_MORE_COLOR = 1.1;
const HIGHLIGHT_LIGHTER = 0.5;
const HIGHLIGHT_PALER = 0.5;
/** The degrees of hue the shadows are turned towards orange, and the highlights towards yellow. */
const SHADOW_WARMER = 16;
const HIGHLIGHT_YELLOWER = 4;
/** How much less colorful the brightest color is than the color of a swatch (hair in a photo is paler than a swatch of the same color). */
const LIGHT_PALER = 0.15;
/**
 * Where in the gradient (0 dark to 1 light) the hair's middle tone is put. Below half, more of the hair sits in the darker half and the
 * lightest strands stand out as highlights, which is the depth of real hair; at half, the hair is mostly the middle color and looks flat.
 */
const MIDDLE_AT = 0.42;
/**
 * The brightest and darkest of the hair's tones that are mapped to the ends of the gradient, as the share of the hair that is darker
 * (`LOW_TONE`) and lighter (`HIGH_TONE`) than them, so a few specks of white or black do not use up the gradient. The hair's tones are
 * spread over the gradient at least `LEAST_RANGE` wide (0 to 1), so hair with hardly any variation is not stretched into noise.
 */
const LOW_TONE = 0.03;
const HIGH_TONE = 0.97;
const LEAST_RANGE = 0.08;
/**
 * The fine detail put back on the hair before its tones are mapped: how wide it is as a fraction of the photo's width (a strand, not a
 * lock), and how strong. Hair in a soft photo has little of it, and the map would show a smooth surface.
 */
const DETAIL_REACH = 0.002;
const DETAIL_STRENGTH = 0.6;

/** The darkest, the middle and the lightest tone (0 to 1) of the hair under a mask, where `LOW_TONE` and `HIGH_TONE` of it are darker and lighter. */
type Tones = { low: number; middle: number; high: number };
/** What `tonesOf` found for each hair, which only changes with the hair, kept while the sliders move. */
const toneCache = new WeakMap<object, Tones>();

/** The tones of the pixels of `image`, counting each by `weights` (the hair mask over it), as the Color blend's luma. */
const tonesOf = (hair: MaskLike, image: { rgba(): ArrayBuffer }, weights: ArrayBuffer): Tones => {
  const known = toneCache.get(hair);
  if (known) return known;
  const pixels = new Uint8Array(image.rgba());
  const weight = new Uint8Array(weights);
  const counts = new Float64Array(256);
  let total = 0;
  for (let at = 0; at < weight.length; at++) {
    if (!weight[at]) continue;
    const light = 0.299 * pixels[at * 4] + 0.587 * pixels[at * 4 + 1] + 0.114 * pixels[at * 4 + 2];
    counts[Math.round(light)] += weight[at];
    total += weight[at];
  }
  /** The tone that this share of the hair is darker than. */
  const darkerThan = (share: number) => {
    let seen = 0;
    for (let tone = 0; tone < 256; tone++) {
      seen += counts[tone];
      if (seen >= share * total) return tone / 255;
    }
    return 1;
  };
  const found = total ? { low: darkerThan(LOW_TONE), middle: darkerThan(0.5), high: darkerThan(HIGH_TONE) } : { low: 0, middle: 0.5, high: 1 };
  toneCache.set(hair, found);
  return found;
};

/**
 * Colors the hair of the person in `context`, or of everyone when it names none, on `sheet`, which becomes a layer over the photo.
 *
 * The hair is turned into its tones (gray, spread so its darkest strands are at the dark end and its lightest at the light end, with its
 * middle tone in the middle) and the tones are mapped through a gradient made from the color: darker and warmer in the shadows, the color
 * in the middle, lighter and paler in the highlights. A gradient is how a colorist makes hair look like hair: it is not one flat color,
 * and it does not depend on how dark the hair was, so dark brown hair becomes blonde as easily as blonde becomes brown. The hair's own
 * strands decide where the shadows and highlights fall. `hue` is 0 to 1 along `Gradient.hue()`, `saturation` is how colorful, and `shift`
 * (-1 darkest to 1 lightest) is how bright the middle color is. How much of it shows is the layer's opacity.
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
    // The settings that make it are part of what it is kept under, so a change to one of them makes it again.
    const kept = `${person?.id ?? ''}|${(others ?? []).join(',')}|${[EDGE_REACH, EDGE_STRENGTH, EDGE_GROW, SKIN_STRENGTH, WISP_BOOST, BAND_REACH, BAND_SPREAD, COLOR_MIX_LEAST, SKIN_AT_EDGE].join(',')}`;
    let foregroundMade: ImageLike | undefined;
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
        // Hair is not skin: where the model is as sure of skin as it is of hair, it is skin (the skin is strengthened first, so a place it is
        // only half sure of counts as skin). Without this the strengthening below, and the edge fitting, spread the hair over the skin
        // around it. Projects saved before the skin was found with the hair have no skin here and keep the hair as found.
        const skin = preloaded['bodyskin|all'] as MaskLike | undefined;
        if (skin) {
          // Skin counts for more than the model says, but not twice as much: that took the pixels along the hairline away from the hair too,
          // and left a thin line of the old hair color between the new hair and the forehead.
          const stronger = own(skin.toImage());
          stronger.mul(SKIN_STRENGTH);
          wanted = own(wanted.minimum(own(own(Mask.fromImage(stronger)).inverted())));
        }
        // The model is unsure about thin wisps and gives them a low value, so the mask is strengthened (its values added to themselves, up
        // to full) and then trimmed back to the real edges by the photo, below.
        for (let boost = 1; boost < WISP_BOOST; boost *= 2) wanted = own(wanted.added(wanted));
        // The pixels along the edge of the hair are a mix of hair and what is behind it, and the model leaves them out, so they stayed the old
        // color as a pale rim. The mask is grown by a pixel or two, and fitting it to the photo then trims it back to the real edge.
        wanted = own(wanted.dilated(Math.max(1, Math.round(sheet.width() * EDGE_GROW))));
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
        // The edge is fitted to the photo, but the pixels along it are a mix of hair and what is behind it, and the model cannot say how much of
        // each. They are decided by color. The sure hair is the mask shrunk, and the sure background is the mask grown and turned over. The color
        // the hair has near each pixel, and the color the background has, are made from each by keeping only the photo's color inside it, blurring
        // it outwards and dividing by the blurred zone (so the blur is an average of the zone's own colors, which fills in under the edge). A pixel in
        // the band between them is then the share of the hair color in its color, and where the two colors are too alike to tell (hair the color of
        // the wall) the fitted edge decides.
        const band = Math.max(2, Math.round(sheet.width() * BAND_REACH));
        const sureHair = own(edge.eroded(band));
        const notBackground = own(edge.dilated(band));
        const colorsOf = (zone: MaskLike) => {
          const colors = own(crop.clone());
          const weight = own(zone.toImage());
          const spread = Math.max(2, Math.round(sheet.width() * BAND_SPREAD));
          keep(keep(blend(weight)).withMode(BlendMode.Multiply)).apply(colors);
          keep(gaussianBlur(spread)).apply(colors);
          keep(gaussianBlur(spread)).apply(weight);
          keep(keep(blend(weight)).withMode(BlendMode.Divide)).apply(colors);
          return colors;
        };
        const byColor = own(
          Mask.fromColorMix(crop, colorsOf(sureHair), colorsOf(own(notBackground.inverted())), edge, COLOR_MIX_LEAST),
        );
        // The hair's own color near each pixel is kept, for the edge pixels to be colored from (see below).
        foregroundMade = colorsOf(sureHair).clone();
        foregroundOrigin.set(foregroundMade, { x, y });
        // Only the band changes: the sure hair stays as it is and nothing is added past the grown edge.
        const full = own(
          own(sureHair.maximum(own(byColor.minimum(notBackground)))).placed(sheet.width(), sheet.height(), { x, y }),
        );
        // Skin right beside the hair is held back harder than the skin under the hair (see `SKIN_AT_EDGE`): it is what the edge fitting spread the
        // hair over, as flat patches of color on the neck and shoulders. A mask that is returned is not freed with the others, so it is a new one.
        if (!skin) return full.maximum(full);
        const hardSkin = own(skin.toImage());
        hardSkin.mul(SKIN_AT_EDGE);
        return full.minimum(own(own(Mask.fromImage(hardSkin)).inverted()));
      }),
    ) as MaskLike | undefined;
    const foreground = derivedFor(preloaded, 'hair|foreground', kept, () => foregroundMade) as ImageLike | undefined;
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
    // How firm the hair's edge is depends on how light the color is. The edge is still a little soft, and a soft fringe tints a light
    // background around the hair, so light colors need a precise edge: the faintest part is cut off and the rest made firmer. Dark colors look
    // more natural with a soft edge, which also keeps fine wisps. Each pixel becomes `(value - cut) * gain`, both growing with the lightness.
    const brightness = Math.min(0.97, Math.max(0.03, HAIR_MIDDLE + shift * HAIR_REACH));
    const edgeLight = Math.min(1, Math.max(0, (brightness - EDGE_LIGHT_FROM) / EDGE_LIGHT_SPAN));
    const firm = keep(keep(region.cropped(part)).toImage());
    firm.sub(EDGE_CUT_SOFT + (EDGE_CUT_PRECISE - EDGE_CUT_SOFT) * edgeLight);
    firm.mul(1 + (EDGE_GAIN_PRECISE - 1) * edgeLight);
    const here = keep(Mask.fromImage(firm));

    // The tones: gray, then each pixel is `(tone - start) / range`, which puts the hair's middle tone in the middle of 0 to 1 and its
    // darkest and lightest strands near the ends. The start is taken away first and the gain applied after, so nothing is cut off by the
    // gain; a start below 0 is added instead (very dark hair).
    const { low, middle, high } = tonesOf(region, image, here.values());
    // The pixels along the edge are a mix of hair and what is behind it, and coloring such a pixel from its own color leaves a gray rim of
    // the old color. So each is laid over the hair's color near it, as much as it is not hair: `hair * share + hair color * (1 - share)`.
    if (foreground) {
      const origin = foregroundOrigin.get(foreground) ?? { x: 0, y: 0 };
      const hairColor = keep(foreground.cropped(part.x - origin.x, part.y - origin.y, part.width, part.height));
      keep(here.inverted()).applyToImage(hairColor);
      keep(keep(blend(hairColor)).withMode(BlendMode.Normal)).apply(image);
    }
    const range = Math.max(high - low, LEAST_RANGE);
    const start = middle - range * MIDDLE_AT;
    keep(grayscale()).apply(image);
    if (Math.abs(start) > 0.004) {
      const offset = keep(Image.newFromColor(part.width, part.height, keep(Color.fromHsv(0, 0, Math.min(1, Math.abs(start))))));
      keep(keep(blend(offset)).withMode(start > 0 ? BlendMode.Subtract : BlendMode.LinearDodge)).apply(image);
    }
    keep(channelMix([1 / range, 0, 0], [0, 1 / range, 0], [0, 0, 1 / range])).apply(image);

    // Brings out the strands before the tones are mapped: the hair laid with a slight blur of itself turned upside down at half strength is
    // gray where the hair is smooth and lighter or darker along a strand, and Linear Light adds that detail back.
    const soft = keep(image.clone());
    keep(gaussianBlur(Math.max(1, Math.round(sheet.width() * DETAIL_REACH)))).apply(soft);
    keep(invert()).apply(soft);
    const detail = keep(image.clone());
    keep(keep(blend(soft)).withOpacity(0.5)).apply(detail);
    keep(keep(keep(blend(detail)).withMode(BlendMode.LinearLight)).withOpacity(DETAIL_STRENGTH)).apply(image);

    // The gradient: the color's shadow, the color and its highlight.
    const degrees = 360 * (1 - hue);
    // A light color is less colorful in hair than it is on a swatch, and a dark yellow is a muddy olive, so the shadows turn towards warm
    // orange-brown and the highlights a little towards yellow, as real hair does.
    const middleColor = saturation * (1 - LIGHT_PALER * brightness);
    const turned = (shiftDegrees: number) => (degrees + shiftDegrees + 360) % 360;
    const colors = [
      Color.fromHsv(turned(-SHADOW_WARMER), Math.min(1, middleColor * SHADOW_MORE_COLOR), brightness * (1 - SHADOW_DARKER)),
      Color.fromHsv(degrees, middleColor, brightness),
      Color.fromHsv(turned(HIGHLIGHT_YELLOWER), middleColor * (1 - HIGHLIGHT_PALER), brightness + (1 - brightness) * HIGHLIGHT_LIGHTER),
    ];
    const gradient = keep(new Gradient(colors.map((color, at) => keep(new ColorStop(keep(color), at / 2)))));
    keep(gradientMap(gradient)).apply(image);

    here.applyToImage(image);
    sheet.painted = { image, x, y };
  });
};
