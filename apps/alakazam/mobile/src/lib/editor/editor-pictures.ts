import { previewBox } from '@/src/state/gestures';
import { previewSourceImage, showingCheckpoint } from '@/src/state/preview';
import { originalImage } from '@/src/state/session';
import { ResizeTarget } from '@abra/core/canvas';
import { resize } from '@abra/core/transform';
import type { AbraImage } from '@alakazam/mobile';
import { Dimensions, PixelRatio } from 'react-native';

// What is on screen. The editor only ever draws a photo shrunk to the preview (a photo that already fits is drawn as it is).
// Every edit is made twice: first on that screen-size picture, which is what the user sees, and then, a moment later, on the
// full-size photo, which is never drawn and only exists for what needs every pixel (the saved file, the blemish tool). A drag
// runs its tool on the same screen-size picture, so releasing the slider shows what the drag showed, with nothing changing
// underneath.
//
// This module holds native images, and it imports nothing from the controls or the replay: saving a control or a tool re-runs
// those and everything that imports them, and what is held here must survive that, not be started again empty.

/** A copy of `source` shrunk to fit within `width` x `height`, or just a copy when it already fits. The caller owns it. */
export function fitted(source: AbraImage, width: number, height: number): AbraImage {
  const copy = source.clone() as AbraImage;
  if (copy.width() > width || copy.height() > height) resize(ResizeTarget.Fit.new({ width, height })).apply(copy);
  return copy;
}

/** A screen-size copy of a full-size image, and what it was made from, so it is only made again when either changes. */
type Shrunk = { source: AbraImage; width: number; height: number; image: AbraImage };
let shrunkBase: Shrunk | null = null;
let shrunkOriginal: Shrunk | null = null;
/** The thumbnail-size copy of the edit base, which every control's preview card starts from. */
let shrunkThumbnail: Shrunk | null = null;
/** The edited screen-size picture that is on screen. Owned here. */
let shown: AbraImage | null = null;

/** The pixel size the editor's live frames are rendered at: the preview box at the screen's density. */
export function previewPixels(): { width: number; height: number } | null {
  const box = previewBox.value;
  if (!box) return null;
  const ratio = PixelRatio.get();
  return { width: Math.round(box.width * ratio), height: Math.round(box.height * ratio) };
}

/** The size the editor draws at: the preview, or the screen until the preview has been measured. */
function drawSize(): { width: number; height: number } {
  const preview = previewPixels();
  if (preview) return preview;
  const screen = Dimensions.get('window');
  const density = PixelRatio.get();
  return { width: Math.round(screen.width * density), height: Math.round(screen.height * density) };
}

/**
 * The screen-size copy of `source` (or a `size` one, for the thumbnails), which stays the same copy until `source` or the size
 * changes. Owned by the cache.
 */
export function shrunkOf(
  slot: 'base' | 'original' | 'thumbnail',
  source: AbraImage,
  size: { width: number; height: number } = drawSize(),
): AbraImage {
  const { width, height } = size;
  const held = slot === 'base' ? shrunkBase : slot === 'original' ? shrunkOriginal : shrunkThumbnail;
  if (held && held.source === source && held.width === width && held.height === height) return held.image;
  held?.image.uniffiDestroy();
  const next: Shrunk = { source, width, height, image: fitted(source, width, height) };
  if (slot === 'base') shrunkBase = next;
  else if (slot === 'original') shrunkOriginal = next;
  else shrunkThumbnail = next;
  return next.image;
}

/** The picture a held before/after preview should show: the untouched original, shrunk to the screen like the edited one. */
export const comparisonImage = (): AbraImage | null => {
  const original = originalImage.value;
  return original ? shrunkOf('original', original) : null;
};

/** Puts `image`, a screen-size picture this module now owns, on screen in place of the last one. */
export function setShown(image: AbraImage) {
  const previous = shown;
  shown = image;
  previewSourceImage.next(showingCheckpoint.value ? (comparisonImage() ?? image) : image);
  if (previous && previous !== image) previous.uniffiDestroy();
}

/** The edited picture that is on screen, or null before there is one. */
export const shownImage = (): AbraImage | null => shown;

/** Shows a saved project's picture, shrunk, while the project loads. The first replay replaces it. */
export function showStandIn(image: AbraImage) {
  const { width, height } = drawSize();
  setShown(fitted(image, width, height));
}

/** Lets go of every screen-size picture, when the session ends. */
export function releasePictures() {
  const images = [shown, shrunkBase?.image, shrunkOriginal?.image, shrunkThumbnail?.image];
  shown = null;
  shrunkBase = null;
  shrunkOriginal = null;
  shrunkThumbnail = null;
  for (const image of images) image?.uniffiDestroy();
}
