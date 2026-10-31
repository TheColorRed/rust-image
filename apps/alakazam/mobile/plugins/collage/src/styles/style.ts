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
  /** Whether the Rotation slider applies. A style whose cells fit edge to edge does not turn them. Defaults to true. */
  rotates?: boolean;
  /** Whether the collage can be cut down to the cells, when the style leaves empty bands because its cells keep their proportions. Adds the Trim Edges option. Defaults to false. */
  trims?: boolean;
  /** Whether the Shadow option applies. A style whose cells touch has no room for one. Defaults to true. */
  shadows?: boolean;
  /** Whether the places touch, so a plain border is one width for every cell and neighbors share it. Defaults to false. */
  sharedBorder?: boolean;
  /** Where a scaled cell stays put: its slot's top-left corner, so it grows down and to the right, or its middle. */
  scaleFrom: 'corner' | 'middle';
  /** Names everything the style's own controls have been set to, so the collage is made again when any of it changes. */
  key(): string;
  /** Adds the style's own controls to the inspector (its columns and rows, say). */
  controls(): void;
  /** Whether the style needs the proportions of every picked picture, so the collage reads them all before the style lays them out. Defaults to false. */
  needsShapes?: boolean;
  /** The cells for `images` pictures in a collage of `size` (width, height). `shapes` is each picture's width over its height, only for a style that `needsShapes`. The same input gives the same cells. */
  layout(images: number, size: [number, number], shapes: number[], border?: { side: number; bottom: number }): Placement[];
  // `border` is the width in pixels of the border every cell shares (its sides and top, and its bottom), for a style with `sharedBorder`.
  // A place then holds its picture and one border across and one down, so the style makes it that much bigger than the picture.
};
