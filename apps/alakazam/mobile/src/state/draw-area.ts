import { BehaviorSubject } from 'rxjs';

/** A point of the photo, in its pixels. */
export type DrawnPoint = { x: number; y: number };

/** What a drawn loop does to an area: starts it over with the loop, adds to it, or takes away from it. */
export type DrawMode = 'new' | 'add' | 'subtract';

/** A change made through the shared area controls, for a tool controller that needs to react to it. */
export type AreaChange = { type: 'draw'; loop: DrawnPoint[]; mode: DrawMode } | { type: 'clear' };

/**
 * An area that a control lets the person draw on the photo. Any control can offer one (a color's area, what to remove, a mask to paint):
 * it describes the area here while it is open, and the Draw area, + and − buttons, the drawing layer over the photo and the outline of the
 * area all work from this, so none of them know which control it is.
 */
export interface AreaTarget {
  /** The control that offers the area. A control clears only its own target. */
  owner: string;
  /** Names the area being drawn. When it changes, the drawing ends, so a loop never lands on the wrong area. */
  key: string;
  /** Whether the area can be drawn now. Drawing waits for a color to be on, for instance. */
  canDraw: boolean;
  /** Start drawing as soon as the area opens, for a control that is nothing but drawing. The Draw area button turns it off to pan and zoom. */
  startsDrawing?: boolean;
  /** Which of the outline colors the area is shown in. */
  colorIndex?: number;
  /** What to tell the person while they draw in each mode. */
  hint: Record<DrawMode, string>;
  /** How opaque the loop is while it is drawn, and the share of the photo a loop must enclose to count. See `DrawAreaOverlay`. */
  opacity?: number;
  minShare?: number;
  /** The Clear button was pressed: the control empties its area. */
  onClear: () => void;
  /** A loop was drawn, in the photo's pixels. The control starts its area over with it, adds it, or takes it away, as `mode` says. */
  onDrawn: (loop: DrawnPoint[], mode: DrawMode) => void;
  /** Called after a draw or clear action, so other parts of the control can react to shared area edits. */
  onChange?: (change: AreaChange) => void;
}

/**
 * One shape of an area: `points` are `[x, y]` fractions of the photo, so it holds at any size. An area is every shape with its `key`,
 * so an area can be several shapes, and a shape wound the other way is a hole in the ones around it (see `areaMask`).
 */
export type AreaRing = { key: string; points: [number, number][] };

/**
 * Every area drawn on the photo, as a list of shapes. The areas of all controls are here, so the outline on the photo and the + and −
 * buttons read any control's area the same way. A control whose area is saved elsewhere (a color's, in the edits) copies it here while it
 * is open.
 */
export const areas = new BehaviorSubject<AreaRing[]>([]);
export const areas$ = areas.asObservable();

/** The shapes of the area `key` in `list`, as rings of `[x, y]` fractions. */
export const areaRings = (list: readonly AreaRing[], key: string): number[][][] =>
  list.filter(ring => ring.key === key).map(ring => ring.points);

/** Replaces the shapes of the area `key`. With no rings the area is gone. */
export const setArea = (key: string, rings: number[][][]) =>
  areas.next([...areas.value.filter(ring => ring.key !== key), ...rings.map(points => ({ key, points: points as [number, number][] }))]);

/** The area the open control offers, or null when it offers none. */
export const areaTarget = new BehaviorSubject<AreaTarget | null>(null);
export const areaTarget$ = areaTarget.asObservable();

/** The `key` of the area being drawn, or null when the drawing is closed. The Draw area button sets it. */
export const areaDrawKey = new BehaviorSubject<string | null>(null);
export const areaDrawKey$ = areaDrawKey.asObservable();
/** What a drawn loop does to the area being drawn. The Draw area, + and − buttons set it. */
export const areaDrawMode = new BehaviorSubject<DrawMode>('new');
export const areaDrawMode$ = areaDrawMode.asObservable();
