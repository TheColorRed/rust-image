import { Rand, inspector } from '@alakazam/plugin';
import { between, dealt } from './randomness';
import { SEED, type CollageStyle } from './style';

/** How many pictures are placed. */
let count = 6;

/** How big each picture is before it is scaled, as a share of the shorter side of the collage. */
const SIZE_SHARE = 0.4;

/** A number of pictures, each at a random place in the collage. The Scale slider grows and shrinks each one about its middle. */
export const randomStyle: CollageStyle = {
  name: 'Random',
  scales: true,
  scaleFrom: 'middle',
  key: () => `random:${count}`,
  controls: () => {
    inspector.number('Images', {
      value: count,
      min: 1,
      max: 100,
      onChange: value => {
        count = Math.round(value);
      },
    });
  },
  layout: (images, [width, height]) => {
    const side = Math.max(1, Math.floor(Math.min(width, height) * SIZE_SHARE));
    const rand = new Rand(SEED);
    return dealt(count, images, rand).map(image => ({
      image,
      x: between(rand, 0, Math.max(0, width - side)),
      y: between(rand, 0, Math.max(0, height - side)),
      width: side,
      height: side,
    }));
  },
};
