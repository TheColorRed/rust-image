import type { PreviewBox } from '@/src/state/gestures';

export type PreviewPoint = { x: number; y: number };

/**
 * Where a screen point is before the photo is turned: the photo turns clockwise by `rotation` radians about the middle of the box,
 * outside its zoom and pan, so this turns the point back the other way about that middle.
 */
export function unrotatePreviewPoint(point: PreviewPoint, box: PreviewBox, rotation: number): PreviewPoint {
  if (rotation === 0) return point;
  const dx = point.x - box.width / 2;
  const dy = point.y - box.height / 2;
  const cos = Math.cos(rotation);
  const sin = Math.sin(rotation);
  return { x: box.width / 2 + cos * dx + sin * dy, y: box.height / 2 - sin * dx + cos * dy };
}

/**
 * Maps a preview-local point through its zoom/pan and contained photo size into source-image pixels. `rotation` is how far the
 * photo is turned clockwise on the screen, in radians.
 */
export function computePreviewImagePoint(
  imageWidth: number,
  imageHeight: number,
  box: PreviewBox,
  scale: number,
  translate: PreviewPoint,
  screenPoint: PreviewPoint,
  rotation = 0,
): PreviewPoint | null {
  if (imageWidth <= 0 || imageHeight <= 0 || scale <= 0) return null;

  const point = unrotatePreviewPoint(screenPoint, box, rotation);
  const localX = (point.x - box.width / 2 - translate.x) / scale + box.width / 2;
  const localY = (point.y - box.height / 2 - translate.y) / scale + box.height / 2;
  const containScale = Math.min(box.width / imageWidth, box.height / imageHeight);
  const displayedWidth = imageWidth * containScale;
  const displayedHeight = imageHeight * containScale;
  const offsetX = (box.width - displayedWidth) / 2;
  const offsetY = (box.height - displayedHeight) / 2;
  const u = (localX - offsetX) / displayedWidth;
  const v = (localY - offsetY) / displayedHeight;
  if (u < 0 || u >= 1 || v < 0 || v >= 1) return null;
  return { x: u * imageWidth, y: v * imageHeight };
}

/**
 * The part of a photo (each side from 0 to 1 of the whole photo: left, top, width, height) that a preview box shows at a zoom and
 * pan. The photo sits contained in the box; the part of the view past its edges is outside 0 to 1.
 */
export function previewViewport(
  photo: { width: number; height: number },
  box: { width: number; height: number },
  zoom: number,
  move: { x: number; y: number },
): [number, number, number, number] {
  const fit = Math.min(box.width / photo.width, box.height / photo.height);
  const size = { width: Math.max(1, photo.width * fit), height: Math.max(1, photo.height * fit) };
  const left = (-box.width / 2 - move.x) / zoom;
  const top = (-box.height / 2 - move.y) / zoom;
  return [
    (left + size.width / 2) / size.width,
    (top + size.height / 2) / size.height,
    box.width / zoom / size.width,
    box.height / zoom / size.height,
  ];
}
