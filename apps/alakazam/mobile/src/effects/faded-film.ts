import { CurveChannel, curves } from '@abra/core/adjustments';
import { blend, BlendMode } from '@abra/core/blend';
import { Curve } from '@abra/core/geometry';
import { type AbraImageLike } from '@alakazam/mobile';
import { owning } from '@/src/lib/native/native-handles';

/** Lifts the blacks by `fade` and rolls off the highlights. At the default of 0.5 the curve is unchanged from before. */
const rgbPoints = (fade: number) => {
  const lift = 90 * fade;
  return [
    { x: 0, y: lift },
    { x: 64, y: 72 + (lift - 45) * 0.5 },
    { x: 192, y: 200 },
    { x: 255, y: 230 },
  ];
};

/**
 * Turns `photo` into a faded film print in place: each channel gets its own S-curve, and the blacks are lifted while the
 * highlights are rolled off, for a matte look. `strength` sets how much of the effect shows, `warmth` (0 cool to 1 warm) sets
 * the blue lift, and `fade` (0 to 1) sets how much the blacks are lifted.
 */
export const fadedFilm = (photo: AbraImageLike, strength: number, warmth: number, fade: number): void =>
  owning(keep => {
    if (strength <= 0) return;
    const film = photo.clone();
    const base = curves();
    const red = Curve.easeInOut(1.2);
    const green = Curve.easeOutIn(1.3);
    const blue = Curve.easeOutIn(1.5 + (0.5 - warmth));
    const withRed = base.withCurve(CurveChannel.Red, red);
    const withGreen = withRed.withCurve(CurveChannel.Green, green);
    const withBlue = withGreen.withCurve(CurveChannel.Blue, blue);
    const toned = withBlue.withPoints(CurveChannel.Rgb, rgbPoints(fade));
    const layer = blend(film);
    const mode = layer.withMode(BlendMode.Normal);
    const opacity = mode.withOpacity(strength);
    keep(film, base, red, green, blue, withRed, withGreen, withBlue, toned, layer, mode, opacity);
    toned.apply(film);
    opacity.apply(photo);
  });
