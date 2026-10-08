import type { PreviewBox } from '@/src/state/gestures';

export type PreviewPoint = { x: number; y: number };

/** Maps a preview-local point through its zoom/pan and contained photo size into source-image pixels. */
export function computePreviewImagePoint(
  imageWidth: number,
  imageHeight: number,
  box: PreviewBox,
  scale: number,
  translate: PreviewPoint,
  point: PreviewPoint,
): PreviewPoint | null {
  if (imageWidth <= 0 || imageHeight <= 0 || scale <= 0) return null;

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
