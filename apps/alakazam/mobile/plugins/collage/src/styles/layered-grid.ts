import { Rand, inspector } from '@alakazam/plugin';
import { dealt, shuffled } from './randomness';
import { SEED, type CollageStyle } from './style';

/** The number of columns and rows. */
let grid: [number, number] = [3, 3];
/** Whether each picture keeps its own proportions, and is not cropped to its cell. */
let keepAspect = true;

/**
 * The cells of a grid, each the size of its place (or, with Keep Aspect Ratio, as big as its picture fits there without cropping), with
 * the pictures in the places in a random order. The Scale slider grows and shrinks each cell about its middle, which is also what it
 * turns about, so the pictures overlap their neighbours on every side.
 */
export const layeredGridStyle: CollageStyle = {
  name: 'Layered Grid',
  scales: true,
  scaleFrom: 'middle',
  key: () => `layered:${grid.join('x')}:${keepAspect}`,
  controls: () => {
    inspector.size('Grid', {
      labels: ['Columns', 'Rows'],
      value: grid,
      min: 1,
      max: 20,
      onChange: value => {
        grid = value;
      },
    });

    inspector.toggle('Keep Aspect Ratio', {
      value: keepAspect,
      onChange: value => {
        keepAspect = value;
      },
    });
  },
  layout: (images, [width, height]) => {
    const [columns, rows] = grid;
    const cellWidth = Math.floor(width / columns);
    const cellHeight = Math.floor(height / rows);
    const rand = new Rand(SEED);
    const places = shuffled(columns * rows, rand);
    return dealt(columns * rows, images, rand).map((image, cell) => ({
      image,
      x: (places[cell] % columns) * cellWidth,
      y: Math.floor(places[cell] / columns) * cellHeight,
      width: cellWidth,
      height: cellHeight,
      keepAspect,
    }));
  },
};
