import { channelMix } from '@abra/core/adjustments';
import { Color } from '@abra/core/color';
import { gaussianBlur } from '@abra/core/filters';
import { type ImageLike } from '@abra/core/image';
import { Mask, type MaskLike } from '@abra/core/mask';
import { derivedFor } from '@/src/lib/editor-preloads';
import { belongsTo, owning } from '@/src/lib/faces';
import { type ToolContext } from '@/src/lib/edit-sections';

/**
 * How wide the softest edge is, as a fraction of the width of the mouth: the Softness slider's 1. Lipstick has a fairly hard
 * edge, so the slider starts low; 0.04 of the mouth's width was too fuzzy.
 */
const MOST_SOFT_EDGE = 0.04;
/** How much each of red, green and blue counts towards how bright a color looks. */
const LUMA = [0.299, 0.587, 0.114];
/**
 * How much of its own brightness the lipstick keeps from the lips, from 0 (none: the lips are only as bright as the color,
 * which is how multiplying paint works) to 1 (all of it: every color is as bright as the lips were). Holding the lips'
 * brightness exactly made dark colors (blue, purple) look pale and chalky, because a real blue lipstick is darker than a lip;
 * a color is brightened or darkened by only the fourth root of how bright it is, so dark colors come out deep and bright ones
 * stay light.
 */
const KEEP_LIP_BRIGHTNESS = 0.25;
/**
 * Strength is curved (to this power) so that a little of it already shows a color; a straight blend of a color into the
 * lips cancels out where the two are far apart in hue (cyan into pink is gray), and that is where the low end of the slider is.
 */
const STRENGTH_CURVE = 0.75;
/**
 * The hue of the lips, in degrees. The farther a lipstick's hue is from it, the more the lips' own color fades to gray as the
 * lipstick comes in, so the two do not mix into mud. A lipstick near it does not need that, and red would only go dull.
 */
const LIP_HUE = 355;
/** How fast the lips' own color fades: at full distance it is gone once the curved strength is a third of the way up. */
const FADE_SPEED = 3;
/** The least of any channel the lipstick has, so black lips still show their shading and shine. */
const CHANNEL_FLOOR = 0.05;
/**
 * A color brighter than this (its own brightness, 0 to 1) makes the lips lighter, not only as bright as they were, so white is
 * white and a pale pink is pale. A vivid color of the same brightness is lifted less (by this fraction of its saturation), or
 * every bright, colorful shade would wash out.
 */
const LIGHTEN_FROM = 0.45;
const LIGHTEN = 0.9;
const LIGHTEN_VIVID = 0.6;

/**
 * Puts lipstick of a hue (0 to 1 along `Gradient.hue()`, which the bar of hues shows: hue 360 at 0, falling to 0 at 1) on the lips of the person in `context`, or
 * of everyone when it names none, from nothing at `amount` 0 to full at 1, as colorful as `saturation` and as bright as `brightness` (both 0 to 1; black is 0 brightness, white is 0 saturation and 1 brightness), with an edge from hard at `softness` 0 to soft at 1.
 */
export const applyLipstick = (
  photo: ImageLike,
  hue: number,
  amount: number,
  softness: number,
  saturation: number,
  brightness: number,
  { preloaded, person }: ToolContext,
) => {
  if (!preloaded || Array.isArray(preloaded)) return;
  owning(keep => {
    // The lips take the lipstick's color with their own shading showing through, as the mixer below says. `blend` goes from
    // the lips as they are (the mixer that changes nothing) to all lipstick.
    const degrees = 360 * (1 - hue);
    const color = Array.from(new Uint8Array(keep(Color.fromHsv(degrees, saturation, brightness)).rgb()), channel =>
      Math.max(CHANNEL_FLOOR, channel / 255),
    );
    const colorBrightness = LUMA[0] * color[0] + LUMA[1] * color[1] + LUMA[2] * color[2];
    const blend = amount ** STRENGTH_CURVE;
    const apart = Math.min((degrees - LIP_HUE + 360) % 360, (LIP_HUE - degrees + 360) % 360) / 180;
    // A color with no hue to speak of (black, white, gray) has nothing in common with the lips' pink, which fades fully.
    const gray = Math.min(1, blend * FADE_SPEED) * Math.max(Math.sqrt(apart), 1 - saturation);
    const lighten =
      1 +
      LIGHTEN * Math.max(0, (colorBrightness - LIGHTEN_FROM) / (1 - LIGHTEN_FROM)) * (1 - LIGHTEN_VIVID * saturation);
    // Each channel of the result is the lips' brightness, times how much of that channel the color has, scaled so that a
    // bright color keeps the lips' brightness and a dark one makes them darker.
    const [mixRed, mixGreen, mixBlue] = color.map((channel, index) =>
      LUMA.map((weight, source) => {
        const own = (1 - gray) * (index === source ? 1 : 0) + gray * weight;
        return (1 - blend) * own + blend * (channel / colorBrightness ** KEEP_LIP_BRIGHTNESS) * lighten * weight;
      }),
    );
    const lipstick = keep(channelMix(mixRed, mixGreen, mixBlue));
    for (const [key, lips] of Object.entries(preloaded)) {
      const [kind, box] = key.split('|');
      if (kind !== 'lipstick' || !box) continue;
      if (!belongsTo(box, photo, person)) continue;
      // The edge only changes with the softness, so it is made once for it and kept while the other sliders move.
      const softened = derivedFor(preloaded, key, softness.toFixed(3), () =>
        owning(own => {
          const radius = (lips as MaskLike).bounds(0)!.width * MOST_SOFT_EDGE * softness;
          if (radius < 0.5) return undefined;
          const blurred = own((lips as MaskLike).toImage());
          own(gaussianBlur(radius)).apply(blurred);
          return Mask.fromImage(blurred);
        }),
      ) as MaskLike | undefined;
      const edge = softened ?? (lips as MaskLike);
      keep(lipstick.withMask(edge)).apply(photo);
    }
  });
};
