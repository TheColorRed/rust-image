import { gradientMap, grayscale, levels } from '@abra/core/adjustments';
import { BlendMode, blend } from '@abra/core/blend';
import { Color } from '@abra/core/color';
import { Fill, ImageRepeat, fillArea } from '@abra/core/fill';
import { ColorStop, Gradient } from '@abra/core/gradient';
import { noise, pinch } from '@abra/core/filters';
import { Area } from '@abra/core/geometry';
import { Channels, Image } from '@abra/core/image';
import { type AbraImageLike } from '@alakazam/mobile';
import { owning } from '@/src/lib/native/native-handles';
import { vignette } from '@/src/effects/vignette';

/** Dark forest green for the deepest shadows. */
const DARK_GREEN: [number, number, number] = [0, 38, 0];
/** Radioactive green for the midtones. */
const BRIGHT_GREEN: [number, number, number] = [0, 255, 51];
/** Near white for the blown out highlights. */
const WHITE_OUT: [number, number, number] = [230, 255, 242];
/** A 1 by 2 pixel RGBA tile: a black top row and a transparent bottom row, repeated across the frame as scanlines. */
const SCANLINE_TILE = new Uint8Array([0, 0, 0, 255, 0, 0, 0, 0]);

export interface NightVisionOptions {
  /** How much of the night-vision look is blended over the photo, 0 to 1. */
  strength: number;
  /** Monochrome grain on the 0 to 1 scale of the image. */
  grain: number;
  /** Gain applied to the luminance before the gamma curve. Lower values darken the photo. */
  gain: number;
  /** Power applied to the luminance. Above 1 darkens the midtones. */
  power: number;
  /** How dark the corners get, 0 to 1. */
  vignette: number;
  /** Pinch amount: negative bulges the frame outward like a scope lens. */
  barrel: number;
  /** How strongly the horizontal scanlines darken the frame, 0 to 1. */
  scanlines: number;
}

export const NIGHT_VISION_DEFAULTS: NightVisionOptions = {
  strength: 1,
  grain: 0.12,
  gain: 1.5,
  power: 1.2,
  vignette: 0.8,
  barrel: -0.1,
  scanlines: 0.2,
};

/**
 * Turns `photo` into a night-vision view in place: the luminance is amplified, grain is added, and the result is mapped
 * to a phosphor green gradient with a dark vignette and a barrel bulge. At a strength of 0 the photo is unchanged and at 1
 * it is fully night vision. Values in between fade from one to the other.
 */
export const nightVision = (photo: AbraImageLike, options: NightVisionOptions = NIGHT_VISION_DEFAULTS): void =>
  owning(keep => {
    const { strength, grain, gain, power, vignette: corners, barrel, scanlines } = options;
    if (strength <= 0) return;
    const darkGreen = keep(Color.fromRgb(...DARK_GREEN));
    const brightGreen = keep(Color.fromRgb(...BRIGHT_GREEN));
    const whiteOut = keep(Color.fromRgb(...WHITE_OUT));
    const gradient = keep(
      new Gradient([
        keep(new ColorStop(darkGreen, 0)),
        keep(new ColorStop(brightGreen, 0.7)),
        keep(new ColorStop(whiteOut, 1)),
      ]),
    );
    const print = keep(photo.clone());
    keep(grayscale()).apply(print);
    // Grain goes on the luminance, before colorizing, so it stays monochrome and is amplified by the tone curve.
    if (grain > 0) keep(noise(grain)).apply(print);
    keep(keep(levels(0, 255 / gain)).withGamma(1 / power)).apply(print);
    keep(gradientMap(gradient)).apply(print);
    if (corners > 0) vignette(print, corners, 0.4);
    if (barrel !== 0) keep(pinch(barrel)).apply(print);
    // The scanlines sit on the curved glass, so they are added after the barrel pass and stay straight.
    if (scanlines > 0) {
      const tile = keep(Image.newFromPixels(1, 2, SCANLINE_TILE.buffer, Channels.Rgba));
      const lines = keep(Image.newFromColor(print.width(), print.height(), keep(Color.transparent())));
      // The tile is one pixel wide, so it repeats on both axes to cover the frame.
      const tileFill = keep(Fill.image(tile).withRepeat(ImageRepeat.Both));
      const area = keep(Area.rect({ x: 0, y: 0 }, { width: print.width(), height: print.height() }));
      keep(fillArea(area, tileFill)).apply(lines);
      keep(keep(keep(blend(lines)).withMode(BlendMode.Overlay)).withOpacity(scanlines)).apply(print);
    }
    keep(keep(blend(print)).withOpacity(strength)).apply(photo);
  });
