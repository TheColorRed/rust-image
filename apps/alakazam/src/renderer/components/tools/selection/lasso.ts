import { SelectionArea } from '@/components/tools/selection/selection-area';

export class LassoSelectionArea extends SelectionArea {
  override mouseMove(): void {
    const [x, y] = this.currentPoint;
    this.addPoint(x, y);
  }
}
