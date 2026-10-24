import type { AbraImage } from '@alakazam/mobile';
import type { PreviewBox } from '@/src/state/gestures';
import { rotate } from '@abra/core/transform';
import { release } from '@/src/lib/native/native-handles';

/** The part of the photo to keep, as fractions of its size: 0 is the left or top edge, 1 the right or bottom edge. */
export interface CropRect {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

export const FULL_CROP: CropRect = { left: 0, top: 0, right: 1, bottom: 1 };

/** The least a side can be kept at, as a fraction of the photo, so the gizmos can always be grabbed apart. */
export const MIN_CROP_SIDE = 0.05;

export const isFullCrop = (rect: CropRect) => rect.left <= 0 && rect.top <= 0 && rect.right >= 1 && rect.bottom >= 1;

/** The pixels a crop keeps from a `width` by `height` photo, never outside it and never empty. */
export function cropPixels(rect: CropRect, width: number, height: number) {
  const left = Math.min(Math.max(Math.round(rect.left * width), 0), width - 1);
  const top = Math.min(Math.max(Math.round(rect.top * height), 0), height - 1);
  const right = Math.min(Math.max(Math.round(rect.right * width), left + 1), width);
  const bottom = Math.min(Math.max(Math.round(rect.bottom * height), top + 1), height);
  return { x: left, y: top, width: right - left, height: bottom - top };
}

/** A new picture of `image` cut to `rect`. The caller owns it. */
export function cropImage(image: AbraImage, rect: CropRect): AbraImage {
  const { x, y, width, height } = cropPixels(rect, image.width(), image.height());
  return image.cropped(x, y, width, height) as AbraImage;
}

/** Cuts `image` to `rect`, turning the picture by `turn` radians first when it is tilted. The image given is left as it was. */
export function cropPicture(image: AbraImage, rect: CropRect, turn = 0): AbraImage {
  if (!turn) return cropImage(image, rect);
  const turned = image.clone() as AbraImage;
  try {
    rotate(turnDegrees(turn)).apply(turned);
    return cropImage(turned, rect);
  } finally {
    release(turned);
  }
}

export type Photo = { width: number; height: number };

/** The shape a crop is held to: `freeform` takes any shape, the others are a width over height. */
export type CropAspect = 'freeform' | 'original' | '1:1' | '4:3' | '16:9';

export const CROP_ASPECTS: { key: CropAspect; label: string }[] = [
  { key: '1:1', label: '1:1' },
  { key: '4:3', label: '4:3' },
  { key: '16:9', label: '16:9' },
  { key: 'freeform', label: 'Freeform' },
  { key: 'original', label: 'Original' },
];

export type CropCorner = 'top-left' | 'top-right' | 'bottom-right' | 'bottom-left';

const clamp = (value: number, low: number, high: number) => Math.min(high, Math.max(low, value));

/** The width over height a crop holds to, or `null` when it may take any shape. `original` is the photo's own shape. */
export function aspectRatio(aspect: CropAspect, photo: Photo): number | null {
  switch (aspect) {
    case 'freeform':
      return null;
    case 'original':
      return photo.width / photo.height;
    case '1:1':
      return 1;
    case '4:3':
      return 4 / 3;
    case '16:9':
      return 16 / 9;
  }
}

type Edges = { left: number; top: number; right: number; bottom: number };

const toEdges = (rect: CropRect, photo: Photo): Edges => ({
  left: rect.left * photo.width,
  top: rect.top * photo.height,
  right: rect.right * photo.width,
  bottom: rect.bottom * photo.height,
});

const fromEdges = (edges: Edges, photo: Photo): CropRect => ({
  left: edges.left / photo.width,
  top: edges.top / photo.height,
  right: edges.right / photo.width,
  bottom: edges.bottom / photo.height,
});

/**
 * Every function below takes `bounds`: the part of the photo the crop may cover. The crop controller passes what the preview shows,
 * so the crop cannot be moved above the box. It defaults to the whole photo.
 */

/** Shrinks `rect` about its middle if it is bigger than `bounds`, then slides it inside, keeping its shape. */
export function containCropRect(rect: CropRect, photo: Photo, bounds: CropRect = FULL_CROP): CropRect {
  const limits = toEdges(bounds, photo);
  const edges = toEdges(rect, photo);
  const width = edges.right - edges.left;
  const height = edges.bottom - edges.top;
  const shrink = Math.min(1, (limits.right - limits.left) / width, (limits.bottom - limits.top) / height);
  const fittedWidth = width * shrink;
  const fittedHeight = height * shrink;
  const centerX = (edges.left + edges.right) / 2;
  const centerY = (edges.top + edges.bottom) / 2;
  const left = clamp(centerX - fittedWidth / 2, limits.left, limits.right - fittedWidth);
  const top = clamp(centerY - fittedHeight / 2, limits.top, limits.bottom - fittedHeight);
  return fromEdges({ left, top, right: left + fittedWidth, bottom: top + fittedHeight }, photo);
}

/** The largest crop of `aspect` that fits in `bounds`, centered where `rect` is now. `null` aspect only keeps `rect` inside `bounds`. */
export function fitCropToAspect(
  rect: CropRect,
  aspect: number | null,
  photo: Photo,
  bounds: CropRect = FULL_CROP,
): CropRect {
  if (aspect === null) return containCropRect(rect, photo, bounds);
  const limits = toEdges(bounds, photo);
  const width = Math.min(limits.right - limits.left, (limits.bottom - limits.top) * aspect);
  const height = width / aspect;
  const centerX = ((rect.left + rect.right) / 2) * photo.width;
  const centerY = ((rect.top + rect.bottom) / 2) * photo.height;
  const left = clamp(centerX - width / 2, limits.left, limits.right - width);
  const top = clamp(centerY - height / 2, limits.top, limits.bottom - height);
  return fromEdges({ left, top, right: left + width, bottom: top + height }, photo);
}

/**
 * Drags one corner of the crop by `dx`, `dy` photo pixels, keeping the opposite corner where it is. With an aspect the crop keeps
 * its shape, following the larger of the two drags. It never reaches past `bounds` or gets below the minimum size.
 */
export function moveCropCorner(
  rect: CropRect,
  corner: CropCorner,
  dx: number,
  dy: number,
  photo: Photo,
  aspect: number | null,
  bounds: CropRect = FULL_CROP,
): CropRect {
  const edges = toEdges(rect, photo);
  const limits = toEdges(bounds, photo);
  const fromLeft = corner.endsWith('left');
  const fromTop = corner.startsWith('top');
  const anchorX = fromLeft ? edges.right : edges.left;
  const anchorY = fromTop ? edges.bottom : edges.top;
  // Which way the dragged corner lies from the anchor.
  const signX = fromLeft ? -1 : 1;
  const signY = fromTop ? -1 : 1;
  const pointX = (fromLeft ? edges.left : edges.right) + dx;
  const pointY = (fromTop ? edges.top : edges.bottom) + dy;
  const wantedX = Math.max(0, (pointX - anchorX) * signX);
  const wantedY = Math.max(0, (pointY - anchorY) * signY);
  const roomX = signX > 0 ? limits.right - anchorX : anchorX - limits.left;
  const roomY = signY > 0 ? limits.bottom - anchorY : anchorY - limits.top;
  const minX = MIN_CROP_SIDE * photo.width;
  const minY = MIN_CROP_SIDE * photo.height;
  let width: number;
  let height: number;
  if (aspect === null) {
    width = clamp(wantedX, minX, roomX);
    height = clamp(wantedY, minY, roomY);
  } else {
    width = clamp(Math.max(wantedX, wantedY * aspect), Math.max(minX, minY * aspect), Math.min(roomX, roomY * aspect));
    height = width / aspect;
  }
  const x = anchorX + signX * width;
  const y = anchorY + signY * height;
  return fromEdges(
    {
      left: Math.min(anchorX, x),
      right: Math.max(anchorX, x),
      top: Math.min(anchorY, y),
      bottom: Math.max(anchorY, y),
    },
    photo,
  );
}

/** Slides the whole crop by `dx`, `dy` photo pixels, staying inside `bounds`. */
export function moveCropArea(
  rect: CropRect,
  dx: number,
  dy: number,
  photo: Photo,
  bounds: CropRect = FULL_CROP,
): CropRect {
  const edges = toEdges(rect, photo);
  const limits = toEdges(bounds, photo);
  const width = edges.right - edges.left;
  const height = edges.bottom - edges.top;
  const left = clamp(edges.left + dx, limits.left, limits.right - width);
  const top = clamp(edges.top + dy, limits.top, limits.bottom - height);
  return fromEdges({ left, top, right: left + width, bottom: top + height }, photo);
}

/** A turn in degrees, from -180 to 180, as Abra's rotate takes it. Positive is clockwise, as on the preview. */
export function turnDegrees(turn: number): number {
  const degrees = ((((turn * 180) / Math.PI) % 360) + 360) % 360;
  return degrees > 180 ? degrees - 360 : degrees;
}

/** The size of the photo once it is turned by `turn` radians with its corners kept, as Abra's rotate makes it. */
export function turnedPhoto(photo: Photo, turn: number): Photo {
  const cos = Math.abs(Math.cos(turn));
  const sin = Math.abs(Math.sin(turn));
  return { width: photo.width * cos + photo.height * sin, height: photo.width * sin + photo.height * cos };
}

/**
 * Whether a crop, given in the turned picture, lies over the photo's own pixels rather than the empty corners the turn leaves. Each
 * corner is turned back about the middle and must land inside the unturned photo.
 */
function insideTurnedPhoto(rect: CropRect, photo: Photo, turn: number): boolean {
  const canvas = turnedPhoto(photo, turn);
  const cos = Math.cos(turn);
  const sin = Math.sin(turn);
  const halfWidth = photo.width / 2 + 1e-6;
  const halfHeight = photo.height / 2 + 1e-6;
  return [
    [rect.left, rect.top],
    [rect.right, rect.top],
    [rect.right, rect.bottom],
    [rect.left, rect.bottom],
  ].every(([fx, fy]) => {
    const x = (fx - 0.5) * canvas.width;
    const y = (fy - 0.5) * canvas.height;
    return Math.abs(cos * x + sin * y) <= halfWidth && Math.abs(-sin * x + cos * y) <= halfHeight;
  });
}

/**
 * Fits a crop to a turned photo: it is held to `bounds` and its shape first, then, if a corner reaches the empty corners the turn leaves,
 * it shrinks about the point nearest the middle until none does.
 */
export function fitCropInTurn(rect: CropRect, photo: Photo, turn: number, bounds: CropRect = FULL_CROP): CropRect {
  const contained = containCropRect(rect, turnedPhoto(photo, turn), bounds);
  if (insideTurnedPhoto(contained, photo, turn)) return contained;
  const anchorX = clamp(0.5, bounds.left, bounds.right);
  const anchorY = clamp(0.5, bounds.top, bounds.bottom);
  const at = (f: number): CropRect => ({
    left: anchorX + (contained.left - anchorX) * f,
    top: anchorY + (contained.top - anchorY) * f,
    right: anchorX + (contained.right - anchorX) * f,
    bottom: anchorY + (contained.bottom - anchorY) * f,
  });
  // Shrinking towards a point keeps the shape, and the feasible part of that line is one run from the point, so bisection finds its end.
  let low = 0;
  let high = 1;
  for (let step = 0; step < 30; step++) {
    const middle = (low + high) / 2;
    if (insideTurnedPhoto(at(middle), photo, turn)) low = middle;
    else high = middle;
  }
  return at(low);
}

/** How many dp one photo pixel is on the preview at `scale`, with the photo contained in `box`. */
export function dpPerPhotoPixel(photo: Photo, box: PreviewBox, scale: number): number {
  return Math.min(box.width / photo.width, box.height / photo.height) * scale;
}

/** A crop frame on the preview, in dp from the box's top-left. It stays put on screen while the photo zooms and pans under it. */
export interface ScreenRect {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

type Pan = { x: number; y: number };

/**
 * Where the turned photo's bounding box sits on the preview at this zoom, pan, and turn. The photo is contained in `box` at its own size
 * and centered, then scaled, turned about its middle, and moved. The box itself stays upright, so the crop frame does too.
 */
export function photoBoundsOnScreen(
  photo: Photo,
  box: PreviewBox,
  scale: number,
  translate: Pan,
  turn = 0,
): ScreenRect {
  const fit = Math.min(box.width / photo.width, box.height / photo.height);
  const canvas = turnedPhoto(photo, turn);
  const halfWidth = (canvas.width * fit * scale) / 2;
  const halfHeight = (canvas.height * fit * scale) / 2;
  const cos = Math.cos(turn);
  const sin = Math.sin(turn);
  const centerX = box.width / 2 + cos * translate.x - sin * translate.y;
  const centerY = box.height / 2 + sin * translate.x + cos * translate.y;
  return {
    left: centerX - halfWidth,
    right: centerX + halfWidth,
    top: centerY - halfHeight,
    bottom: centerY + halfHeight,
  };
}

/**
 * The lowest zoom at which the whole turned photo fits above the gestures row (`cover` tall at the bottom of `box`), so every edge of
 * the photo is in reach of a handle. It is 1 when the photo already fits there.
 */
export function cropZoomFloor(photo: Photo, box: PreviewBox, cover: number, turn: number): number {
  const fit = Math.min(box.width / photo.width, box.height / photo.height);
  const canvas = turnedPhoto(photo, turn);
  return Math.min(1, box.width / (canvas.width * fit), (box.height - cover) / (canvas.height * fit));
}

/** The pan that centres the turned photo in the area above the gestures row, so it can be shown whole at the zoom floor. */
export function panToCentreAboveBar(cover: number, turn: number): Pan {
  return { x: -(cover / 2) * Math.sin(turn), y: -(cover / 2) * Math.cos(turn) };
}

/** Where a crop sits on the preview at this zoom, pan, and turn. */
export function cropToScreen(
  rect: CropRect,
  photo: Photo,
  box: PreviewBox,
  scale: number,
  translate: Pan,
  turn = 0,
): ScreenRect {
  const bounds = photoBoundsOnScreen(photo, box, scale, translate, turn);
  const width = bounds.right - bounds.left;
  const height = bounds.bottom - bounds.top;
  return {
    left: bounds.left + rect.left * width,
    right: bounds.left + rect.right * width,
    top: bounds.top + rect.top * height,
    bottom: bounds.top + rect.bottom * height,
  };
}

/** The part of the turned photo that a frame on the preview covers at this zoom, pan, and turn, clamped to the photo. */
export function screenToCrop(
  frame: ScreenRect,
  photo: Photo,
  box: PreviewBox,
  scale: number,
  translate: Pan,
  turn = 0,
): CropRect {
  const bounds = photoBoundsOnScreen(photo, box, scale, translate, turn);
  const width = bounds.right - bounds.left;
  const height = bounds.bottom - bounds.top;
  const fraction = (value: number, origin: number, size: number) => clamp((value - origin) / size, 0, 1);
  return {
    left: fraction(frame.left, bounds.left, width),
    top: fraction(frame.top, bounds.top, height),
    right: fraction(frame.right, bounds.left, width),
    bottom: fraction(frame.bottom, bounds.top, height),
  };
}

/**
 * The zoom and pan that make a frame on screen fill `fill` of `area`, centred in it, as "Circle to zoom" does. `area` defaults to the
 * whole box. The frame's middle is found on the unzoomed, unturned photo first, then moved to the middle of the area. `null` when that
 * would not zoom in.
 */
export function zoomToFrame(
  frame: ScreenRect,
  box: PreviewBox,
  scale: number,
  translate: Pan,
  rotation: number,
  fill: number,
  maxScale: number,
  area: ScreenRect = { left: 0, top: 0, right: box.width, bottom: box.height },
): { scale: number; x: number; y: number } | null {
  const width = frame.right - frame.left;
  const height = frame.bottom - frame.top;
  if (width <= 0 || height <= 0) return null;
  const next = Math.min(
    maxScale,
    scale * Math.min(((area.right - area.left) * fill) / width, ((area.bottom - area.top) * fill) / height),
  );
  if (next <= scale) return null;
  return centreFrameAt(frame, box, scale, translate, rotation, area, next);
}

/**
 * Zooms the preview out, if needed, so a frame that a corner drag has pushed past `area` fits inside it at its own size, centred there.
 * Returns null when the frame is already inside `area`.
 */
export function zoomOutToFrame(
  frame: ScreenRect,
  box: PreviewBox,
  scale: number,
  translate: Pan,
  rotation: number,
  minScale: number,
  area: ScreenRect,
): { scale: number; x: number; y: number } | null {
  const width = frame.right - frame.left;
  const height = frame.bottom - frame.top;
  if (width <= 0 || height <= 0) return null;
  const inside =
    frame.left >= area.left && frame.right <= area.right && frame.top >= area.top && frame.bottom <= area.bottom;
  if (inside) return null;
  const fit = Math.min(1, (area.right - area.left) / width, (area.bottom - area.top) / height);
  return centreFrameAt(frame, box, scale, translate, rotation, area, Math.max(minScale, scale * fit));
}

/**
 * The pan that keeps the point at `fraction` of the turned photo on the same spot of the preview when the zoom changes from `scale` to
 * `next`, so the view zooms around that point.
 */
export function panToKeepPoint(
  fraction: { x: number; y: number },
  photo: Photo,
  box: PreviewBox,
  scale: number,
  translate: Pan,
  turn: number,
  next: number,
): Pan {
  const canvas = turnedPhoto(photo, turn);
  const fit = Math.min(box.width / photo.width, box.height / photo.height);
  const offsetX = (fraction.x - 0.5) * canvas.width * fit * (scale - next);
  const offsetY = (fraction.y - 0.5) * canvas.height * fit * (scale - next);
  const cos = Math.cos(turn);
  const sin = Math.sin(turn);
  return { x: translate.x + cos * offsetX + sin * offsetY, y: translate.y - sin * offsetX + cos * offsetY };
}

/**
 * Slides the photo, without zooming, so a frame a corner drag has pushed past `area` moves back inside it at its own size. Returns the
 * new pan, or `null` when the frame is already inside `area` or is too big to fit in it at this zoom.
 */
export function slideFrameInto(
  frame: ScreenRect,
  scale: number,
  translate: Pan,
  rotation: number,
  area: ScreenRect,
): Pan | null {
  const width = frame.right - frame.left;
  const height = frame.bottom - frame.top;
  if (width > area.right - area.left || height > area.bottom - area.top) return null;
  const dx = frame.left < area.left ? area.left - frame.left : frame.right > area.right ? area.right - frame.right : 0;
  const dy = frame.top < area.top ? area.top - frame.top : frame.bottom > area.bottom ? area.bottom - frame.bottom : 0;
  if (dx === 0 && dy === 0) return null;
  const cos = Math.cos(rotation);
  const sin = Math.sin(rotation);
  return { x: translate.x + cos * dx + sin * dy, y: translate.y - sin * dx + cos * dy };
}

/** The pan and zoom that put `frame` in the middle of `area` at `next` zoom, with the photo under it staying where it is. */
function centreFrameAt(
  frame: ScreenRect,
  box: PreviewBox,
  scale: number,
  translate: Pan,
  rotation: number,
  area: ScreenRect,
  next: number,
): { scale: number; x: number; y: number } {
  const cos = Math.cos(rotation);
  const sin = Math.sin(rotation);
  const x = (frame.left + frame.right) / 2 - box.width / 2;
  const y = (frame.top + frame.bottom) / 2 - box.height / 2;
  const onPhotoX = (cos * x + sin * y - translate.x) / scale;
  const onPhotoY = (-sin * x + cos * y - translate.y) / scale;
  const targetX = (area.left + area.right) / 2 - box.width / 2;
  const targetY = (area.top + area.bottom) / 2 - box.height / 2;
  return {
    scale: next,
    x: cos * targetX + sin * targetY - onPhotoX * next,
    y: -sin * targetX + cos * targetY - onPhotoY * next,
  };
}

/**
 * Keeps the photo covering the crop frame: zooms in if the photo would be too small to cover it, and pans it so no edge of the frame
 * shows past the photo.
 */
export function coverCropFrame(
  frame: ScreenRect,
  photo: Photo,
  box: PreviewBox,
  scale: number,
  translate: Pan,
): { scale: number; x: number; y: number } {
  const fit = Math.min(box.width / photo.width, box.height / photo.height);
  const needed = Math.max(
    (frame.right - frame.left) / (photo.width * fit),
    (frame.bottom - frame.top) / (photo.height * fit),
  );
  const covering = Math.max(scale, needed);
  const halfWidth = (photo.width * fit * covering) / 2;
  const halfHeight = (photo.height * fit * covering) / 2;
  const centerX = box.width / 2;
  const centerY = box.height / 2;
  const pan = (value: number, low: number, high: number) => (low > high ? (low + high) / 2 : clamp(value, low, high));
  return {
    scale: covering,
    x: pan(translate.x, frame.right - centerX - halfWidth, frame.left - centerX + halfWidth),
    y: pan(translate.y, frame.bottom - centerY - halfHeight, frame.top - centerY + halfHeight),
  };
}
