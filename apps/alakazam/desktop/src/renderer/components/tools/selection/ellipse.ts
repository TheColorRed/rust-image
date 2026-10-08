import { SelectionArea } from '@/components/tools/selection/selection-area';

export class EllipseSelectionArea extends SelectionArea {
  /**
   * Draws a ellipse within a bounding box defined by top-left corner, width, and height.
   * @param topX The x-coordinate of the top-left corner.
   * @param topY The y-coordinate of the top-left corner.
   * @param widthX The width of the bounding box.
   * @param widthY The height of the bounding box.
   * @param numPoints The number of points to generate along the ellipse.
   * @returns An array of points representing the ellipse.
   */
  private getEllipsisPoints(topX: number, topY: number, widthX: number, widthY: number, numPoints: number) {
    const points: [number, number][] = [];
    for (let i = 0; i < numPoints; i++) {
      const angle = (i / numPoints) * 2 * Math.PI;
      const x = topX + widthX / 2 + (widthX / 2) * Math.cos(angle);
      const y = topY + widthY / 2 + (widthY / 2) * Math.sin(angle);
      points.push([x, y]);
    }
    return points;
  }

  override mouseMove(): void {
    const [x, y] = this.currentPoint;
    const points: [number, number][] = [];
    const sx = this.startPoint?.[0] ?? 0;
    const sy = this.startPoint?.[1] ?? 0;
    const topX = Math.min(sx, x);
    const topY = Math.min(sy, y);
    const w = Math.abs(x - sx);
    const h = Math.abs(y - sy);
    this.getEllipsisPoints(topX, topY, w, h, 64).forEach(p => points.push(p));
    this.setPoints(points);
  }
}
