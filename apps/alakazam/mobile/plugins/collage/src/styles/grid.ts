import { inspector, seededRandom } from '@alakazam/sdk';
import { dealt } from './randomness';
import { SEED, type CollageStyle } from './style';

/** The number of columns and rows. */
let grid: [number, number] = [2, 2];

/** Evenly sized cells, in rows and columns, that fill the collage. A picture that does not fit its cell is cropped. */
export const gridStyle: CollageStyle = {
  name: 'Grid',
  scales: false,
  scaleFrom: 'middle',
  key: () => `grid:${grid.join('x')}`,
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
  },
  layout: (images, [width, height]) => {
    const [columns, rows] = grid;
    const cellWidth = Math.floor(width / columns);
    const cellHeight = Math.floor(height / rows);
    return dealt(columns * rows, images, seededRandom(SEED)).map((image, cell) => ({
      image,
      x: (cell % columns) * cellWidth,
      y: Math.floor(cell / columns) * cellHeight,
      width: cellWidth,
      height: cellHeight,
    }));
  },
};
