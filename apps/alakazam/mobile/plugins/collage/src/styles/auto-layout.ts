import { inspector, seededRandom, type Rand } from '@alakazam/sdk';
import { dealt } from './randomness';
import { SEED, type CollageStyle, type Placement } from './style';

/** How many pictures are placed, or 0 for each picked picture once. */
let count = 0;
/** Which of the arrangements the pictures are in; each number gives its own order and cuts, and the same number always gives the same collage. */
let seed = 0;
/** How many arrangements there are to step through. Next after the last goes back to the first. */
const SEEDS = 100;

/** A picture, or two parts side by side (`across`) or one over the other. Its `aspect` is its width over its height. */
type Part = { aspect: number; image: number } | { aspect: number; across: boolean; first: Part; second: Part };

/** How far two shapes are from each other, as a ratio: twice as wide and half as wide are the same distance from the same shape. */
const apart = (a: number, b: number) => Math.abs(Math.log(a / b));

/** How much worse than the best split another may be and still be picked by chance, so the pictures do not fall into even rows. */
const ROOM = 0.2;

/**
 * Cuts the pictures `from` up to `to` (in the order picked) into two parts, side by side or one over the other, and does the same to
 * each part, until every part is a picture. The cut is chosen so the part is near the `target` shape, with a little chance among the
 * cuts that are near enough, so the parts differ in size. A part's shape is worked out from the pictures in it once it is cut, and a
 * group of pictures is first judged as if it were one picture of their average shape.
 */
const cut = (aspects: number[], from: number, to: number, target: number, rand: Rand): Part => {
  if (to - from === 1) return { aspect: aspects[from], image: from };
  const average = (start: number, end: number) =>
    Math.exp(aspects.slice(start, end).reduce((sum, each) => sum + Math.log(each), 0) / (end - start));
  const options: { at: number; across: boolean; off: number }[] = [];
  for (let at = from + 1; at < to; at++) {
    const [one, two] = [average(from, at), average(at, to)];
    options.push({ at, across: true, off: apart(one + two, target) });
    options.push({ at, across: false, off: apart(1 / (1 / one + 1 / two), target) });
  }
  const best = Math.min(...options.map(each => each.off));
  const near = options.filter(each => each.off <= best + ROOM);
  const { at, across } = near[Math.floor(rand.next() * near.length)];
  // Each side is as big as its share of the pictures, so it is given the shape of that share of the part.
  const share = (at - from) / (to - from);
  const [firstTarget, secondTarget] = across
    ? [target * share, target * (1 - share)]
    : [target / share, target / (1 - share)];
  const first = cut(aspects, from, at, firstTarget, rand);
  const second = cut(aspects, at, to, secondTarget, rand);
  return {
    aspect: across ? first.aspect + second.aspect : 1 / (1 / first.aspect + 1 / second.aspect),
    across,
    first,
    second,
  };
};

/** Puts a part with its top-left corner at (x, y) and the given height, and its pictures in `placements`. */
const place = (
  part: Part,
  x: number,
  y: number,
  height: number,
  placements: Placement[],
  [offsetX, offsetY]: [number, number],
) => {
  if ('image' in part) {
    placements.push({
      image: part.image,
      x: Math.round(offsetX + x),
      y: Math.round(offsetY + y),
      width: Math.max(1, Math.round(part.aspect * height)),
      height: Math.max(1, Math.round(height)),
      keepAspect: true,
    });
    return;
  }
  if (part.across) {
    place(part.first, x, y, height, placements, [offsetX, offsetY]);
    place(part.second, x + part.first.aspect * height, y, height, placements, [offsetX, offsetY]);
  } else {
    // One over the other they share a width, so each is as tall as that width is for its shape.
    const width = part.aspect * height;
    const firstHeight = width / part.first.aspect;
    place(part.first, x, y, firstHeight, placements, [offsetX, offsetY]);
    place(part.second, x, y + firstHeight, width / part.second.aspect, placements, [offsetX, offsetY]);
  }
};

/**
 * The picked pictures in a random order (each used once before any is used again), at their own proportions and as big as fits in the collage. The pictures are cut into
 * groups that fit together, so rows and columns differ in size and the edges do not line up row by row.
 */
export const autoLayoutStyle: CollageStyle = {
  name: 'Auto',
  scales: false,
  rotates: false,
  shadows: false,
  trims: true,
  sharedBorder: true,
  scaleFrom: 'middle',
  needsShapes: true,
  key: () => `auto:${count}:${seed}`,
  controls: () => {
    inspector.slider('Shuffle', {
      min: 0,
      max: SEEDS - 1,
      step: 1,
      // The pictures are laid out again once the slider stops, not on every move.
      rebuild: true,
      value: seed,
      onChange: value => {
        seed = Math.round(value);
      },
    });
    // Buttons for stepping through the arrangements, so there is no need to find a number with the slider.
    inspector.select('Arrangement', {
      options: ['Previous', 'Next', 'Random'],
      momentary: true,
      onChange: choice => {
        if (choice === 'Random') {
          // Any number but the one showing.
          const other = Math.floor(Math.random() * SEEDS);
          seed = other === seed ? (other + 1) % SEEDS : other;
        } else {
          seed = (seed + (choice === 'Next' ? 1 : SEEDS - 1)) % SEEDS;
        }
      },
    });
    inspector.number('Images', {
      value: count,
      min: 0,
      max: 100,
      onChange: value => {
        count = Math.round(value);
      },
    });
  },
  layout: (images, [width, height], shapes, border) => {
    if (images === 0) return [];
    const rand = seededRandom(`${SEED}:${seed}`);
    // Which picture goes in each place, in a random order that uses every picture before one is used again.
    const order = dealt(count > 0 ? count : images, images, rand);
    // A place holds its picture and one border across and one down, so its shape is the picture's with the border added at the size an
    // average picture will be.
    const average = Math.sqrt((width * height) / order.length) - (border?.bottom ?? 0);
    const placeShape = (shape: number) =>
      border ? (shape * average + border.side) / (average + border.bottom) : shape;
    const whole = cut(
      order.map(image => placeShape(shapes[image])),
      0,
      order.length,
      width / height,
      rand,
    );
    // The whole is as big as fits, in the middle.
    const wholeHeight = Math.min(height, width / whole.aspect);
    const placements: Placement[] = [];
    place(whole, 0, 0, wholeHeight, placements, [(width - wholeHeight * whole.aspect) / 2, (height - wholeHeight) / 2]);
    // The parts know their place in the dealt order, so each is given the picture dealt to it.
    return placements.map(each => ({ ...each, image: order[each.image] }));
  },
};
