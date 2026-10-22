import { type TimelineChange } from '@/src/lib/project/timeline';
import { cropPicture } from '@/src/lib/tools/crop';
import { previewBox } from '@/src/state/gestures';
import { timeline } from '@/src/state/history';
import { previewSourceImage, showingCheckpoint } from '@/src/state/preview';
import { originalImage } from '@/src/state/session';
import { ResizeTarget } from '@abra/core/canvas';
import { Mask, TransformAlgorithm, type MaskLike } from '@abra/core/mask';
import { resize } from '@abra/core/transform';
import type { AbraImage } from '@alakazam/mobile';
import { Dimensions, PixelRatio } from 'react-native';
import { release } from '@/src/lib/native/native-handles';

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
/** The screen-size original cut by the applied crops, and which crop steps (by identity) and shrunk original it was made from. */
let croppedOriginal: { source: AbraImage; crops: TimelineChange[]; image: AbraImage } | null = null;
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
  release(held?.image);
  const next: Shrunk = { source, width, height, image: fitted(source, width, height) };
  if (slot === 'base') shrunkBase = next;
  else if (slot === 'original') shrunkOriginal = next;
  else shrunkThumbnail = next;
  return next.image;
}

/** The crop steps that are applied (before the undo cursor), in order. Each one cuts what the one before it left. */
export const appliedCrops = () => {
  const { entries, cursor } = timeline.value;
  return entries.slice(0, cursor).flatMap(({ change }) => (change.type === 'crop' ? [change] : []));
};

/** Names the applied crops, so something made for them can be kept until they change. */
export const cropKey = () => appliedCrops().map(({ left, top, right, bottom, turn }) => `${left},${top},${right},${bottom},${turn ?? 0}`).join('|');

/**
 * A copy of `mask`, which was found in the uncropped original, cut by the applied crops (turned first when a crop is tilted) and
 * resized to `width` x `height` when given (the size the crops leave otherwise), so it lies on the cropped photo like the edit
 * base does. The caller frees it.
 */
export function maskInEditFrame(mask: MaskLike, width?: number, height?: number): MaskLike {
  let image = mask.toImage() as AbraImage;
  for (const crop of appliedCrops()) {
    const next = cropPicture(image, crop, crop.turn);
    release(image);
    image = next;
  }
  const framed = Mask.fromImage(image);
  release(image);
  if (width === undefined || height === undefined) return framed;
  const sized = framed.resized(width, height, TransformAlgorithm.Bilinear);
  release(framed);
  return sized;
}

/**
 * The picture a held before/after preview should show: the original, shrunk to the screen like the edited one and cut by
 * the crops that are applied, so it is framed like the edited picture and only the other edits differ.
 */
export const comparisonImage = (): AbraImage | null => {
  const original = originalImage.value;
  if (!original) return null;
  const crops = appliedCrops();
  const shrunk = shrunkOf('original', original);
  if (!crops.length) return shrunk;
  if (croppedOriginal && croppedOriginal.source === shrunk && croppedOriginal.crops.every((crop, i) => crop === crops[i]) && croppedOriginal.crops.length === crops.length) {
    return croppedOriginal.image;
  }
  let image = shrunk;
  for (const crop of crops) {
    const next = cropPicture(image, crop, crop.turn);
    if (image !== shrunk) release(image);
    image = next;
  }
  release(croppedOriginal?.image);
  croppedOriginal = { source: shrunk, crops, image };
  return image;
};

/** Puts `image`, a screen-size picture this module now owns, on screen in place of the last one. */
export function setShown(image: AbraImage) {
  const previous = shown;
  shown = image;
  previewSourceImage.next(showingCheckpoint.value ? (comparisonImage() ?? image) : image);
  if (previous && previous !== image) release(previous);
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
  const images = [shown, shrunkBase?.image, shrunkOriginal?.image, shrunkThumbnail?.image, croppedOriginal?.image];
  shown = null;
  croppedOriginal = null;
  shrunkBase = null;
  shrunkOriginal = null;
  shrunkThumbnail = null;
  for (const image of images) release(image);
}
