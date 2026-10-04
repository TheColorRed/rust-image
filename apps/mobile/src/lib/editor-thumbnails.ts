import { AbraImage } from '@alakazam/mobile';
import { PixelRatio } from 'react-native';
import { applyAction, type EditSection } from '@/src/lib/edit-sections';

/** Display size (dp) of a one-tap control's preview thumbnail. */
export const THUMBNAIL_SIZE = 60;

/**
 * Builds a thumbnail per control in `section` (the image with that control's effect applied),
 * against a small downscaled copy rather than the full-resolution image so applying up to a
 * handful of effects stays cheap regardless of the source photo's size.
 */
export function buildControlThumbnails(image: AbraImage, section: EditSection) {
  const size = Math.round(THUMBNAIL_SIZE * PixelRatio.get());
  const basePixels = image.preview(size, size);
  const thumbnails: Record<string, string> = {};
  for (const control of section.controls) {
    // Each control gets its own copy of the base pixels: the JSI bridge under `fromRgba` isn't
    // documented as a read-only, non-detaching borrow, and a shared buffer would explain a single
    // control's thumbnail occasionally coming out blank while its neighbors are fine.
    const thumbnailImage = AbraImage.fromRgba(basePixels.width, basePixels.height, basePixels.data.slice(0)) as AbraImage;
    // One control's effect throwing must not abort the loop: that would skip every later
    // thumbnail and look, from the UI, like "this one control is just blank."
    try {
      if (control.kind === 'action') applyAction(control, thumbnailImage);
      // Read dimensions back rather than assuming they match the base: Rotate Left/Right swap them.
      const width = thumbnailImage.width();
      const height = thumbnailImage.height();
      const pixels = thumbnailImage.rgba();
      if (pixels.byteLength !== width * height * 4) {
        console.warn(`[edit] thumbnail for "${control.key}" has ${pixels.byteLength} bytes for ${width}x${height} RGBA — skipping.`);
        continue;
      }
      thumbnails[control.key] = thumbnailImage.pngDataUri();
    } catch (error) {
      console.warn(`[edit] building thumbnail for "${control.key}" threw:`, error);
    } finally {
      thumbnailImage.uniffiDestroy();
    }
  }
  return thumbnails;
}
