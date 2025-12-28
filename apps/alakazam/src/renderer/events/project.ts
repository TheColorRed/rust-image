import { Subject } from 'rxjs';

/**
 * Events related to the gizmos canvas overlaying the main project canvas.
 * Used for drawing selection rectangles, transformation handles, etc.
 */
export const canvasGizmos = new Subject<{
  action: 'clear' | 'draw';
  callback?: (ctx: CanvasRenderingContext2D) => void;
}>();
/**
 * Events related to mouse interactions on the main project canvas.
 */
export const canvasMouse = new Subject<{
  x: number;
  y: number;
  type: 'down' | 'move' | 'up';
  isMouseDown: boolean;
  event: MouseEvent;
}>();
/**
 * Events related to the project cursor state.
 */
export const projectCursor = new Subject<{ cursor: string; origin: [number, number] }>();
/**
 * Observable streams for external subscription.
 */
export const canvasMouse$ = canvasMouse.asObservable();
/**
 * Observable stream for gizmos events.
 */
export const canvasGizmos$ = canvasGizmos.asObservable();
/**
 * Observable stream for project cursor events.
 */
export const projectCursor$ = projectCursor.asObservable();
