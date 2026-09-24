import { SelectionArea } from '@/components/tools/selection/selection-area';

export class RectSelectionArea extends SelectionArea {
  override mouseMove(): void {
    const [x, y] = this.currentPoint;
    const points = [
      [this.startPoint?.[0] ?? 0, this.startPoint?.[1] ?? 0],
      [x, this.startPoint?.[1] ?? 0],
      [x, y],
      [this.startPoint?.[0] ?? 0, y],
    ] as [number, number][];
    this.setPoints(points);
  }
}
