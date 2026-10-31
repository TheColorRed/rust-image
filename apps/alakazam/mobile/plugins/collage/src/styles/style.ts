/** Where one picture goes in the collage and how big its cell is, in pixels of the collage. */
export type Placement = {
  /** Which of the picked pictures goes in the cell. */
  image: number;
  x: number;
  y: number;
  width: number;
  height: number;
  /** Keeps the picture's own proportions instead of cropping it to the cell: the cell shrinks to the largest picture that fits and sits in the middle of its place. Defaults to false. */
  keepAspect?: boolean;
};

/** What every style seeds its random numbers with, so a collage made again looks the same. */
export const SEED = 'alakazam';

/**
 * A way to lay the pictures out. Each style lives in its own file and says only where the cells go; the collage makes the canvas from
 * them, so a new style is a new file and one line in the list of styles.
 */
export type CollageStyle = {
  /** What the style is called in the style picker. */
  name: string;
  /** Whether the Scale slider applies. A grid has cells that fill their places exactly, so it does not. */
  scales: boolean;
  /** Where a scaled cell stays put: its slot's top-left corner, so it grows down and to the right, or its middle. */
  scaleFrom: 'corner' | 'middle';
  /** Names everything the style's own controls have been set to, so the collage is made again when any of it changes. */
  key(): string;
  /** Adds the style's own controls to the inspector (its columns and rows, say). */
  controls(): void;
  /** The cells for `images` pictures in a collage of `size` (width, height). The same input gives the same cells. */
  layout(images: number, size: [number, number]): Placement[];
};
