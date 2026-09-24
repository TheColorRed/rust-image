import { filter, fromEvent, pipe, Subject } from 'rxjs';

/**
 * Observable stream for mouse wheel events on the document body.
 */
export const mouseWheel$ = fromEvent(document.body, 'wheel').pipe(filter((e): e is WheelEvent => true));
/**
 * Observable stream for mouse move events on the document body.
 */
export const mouseMove$ = fromEvent(document.body, 'mousemove').pipe(filter((e): e is MouseEvent => true));
/**
 * Observable stream for mouse move events with buttons pressed on the document body.
 */
export const mouseMoveWithLeftButtonDown$ = fromEvent(document.body, 'mousemove').pipe(
  filter((e): e is MouseEvent => true),
  filter((e): e is MouseEvent => e.buttons === 1),
);

/**
 * Observable stream for mouse down events on the document body.
 */
export const mouseDown$ = fromEvent(document.body, 'mousedown').pipe(filter((e): e is MouseEvent => true));
/**
 * Observable stream for mouse up events on the document body.
 */
export const mouseUp$ = fromEvent(document.body, 'mouseup').pipe(filter((e): e is MouseEvent => true));

export const cursor = new Subject<{
  cursor: string;
  origin: [number, number];
  size: [number, number] | number;
}>();

export const cursor$ = cursor.asObservable();

/**
 * Checks if the mouse event occurred on the primary project canvas.
 * @param e MouseEvent to check
 */
export const isOnProjectCanvas = function () {
  return pipe(
    filter((e: MouseEvent) => {
      const target = e.target as HTMLElement;
      return target.hasAttribute('data-project-canvas');
    }),
  );
};
