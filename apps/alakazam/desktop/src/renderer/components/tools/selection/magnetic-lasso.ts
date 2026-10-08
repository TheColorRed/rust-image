import { SelectionArea } from '@/components/tools/selection/selection-area';

export class MagneticLassoSelectionArea extends SelectionArea {
  override mouseMove(): void {
    const [x, y] = this.currentPoint;
  }
}
