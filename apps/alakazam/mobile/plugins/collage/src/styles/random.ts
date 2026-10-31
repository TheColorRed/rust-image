import { inspector, seededRandom } from '@alakazam/sdk';
import { dealt } from './randomness';
import { SEED, type CollageStyle } from './style';

/** How many pictures are placed. */
let count = 6;
/** Whether each picture keeps its own proportions, and is not cropped to its square. */
let keepAspect = true;

/** The smallest and the biggest a picture can be before it is scaled, as a share of the shorter side of the collage. Each picture gets a size between them. */
const MIN_SHARE = 0.16;
const MAX_SHARE = 0.3;

/**
 * How far a picture may hang over an edge of the collage, as a share of its own side. Pictures kept wholly inside can only have their
 * middles in the inner part of the collage, which leaves the edges bare and crowds the center.
 */
const OVERHANG = 0.25;

/** How many places are tried for each picture before the best one is kept. More tries spread the pictures more evenly. */
const TRIES = 40;

/**
 * A number of pictures of different sizes, scattered over the collage. With Keep Aspect Ratio, a picture is as big as fits in its square
 * without cropping, so its longer side is the square's side. The Scale slider grows and shrinks each one about its middle.
 *
 * Each picture is put at the best of a number of random places: the one farthest from the pictures already placed, counting a big
 * picture as needing more room than a small one. So the pictures spread over the whole collage and overlap as little as the count
 * allows, with no two clumped together. The biggest are placed first, so the small ones lie on top of them and are never hidden.
 */
export const randomStyle: CollageStyle = {
  name: 'Random',
  scales: true,
  scaleFrom: 'middle',
  key: () => `random:${count}:${keepAspect}`,
  controls: () => {
    inspector.number('Images', {
      value: count,
      min: 1,
      max: 100,
      onChange: value => {
        count = Math.round(value);
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
    const rand = seededRandom(SEED);
    const shorter = Math.min(width, height);
    const pictures = dealt(count, images, rand)
      .map(image => ({ image, side: Math.max(1, Math.round(shorter * (MIN_SHARE + (MAX_SHARE - MIN_SHARE) * rand.next()))) }))
      .sort((a, b) => b.side - a.side);

    const placed: { x: number; y: number; side: number }[] = [];
    for (const { image, side } of pictures) {
      // The middle stays far enough in that the picture hangs over an edge by no more than `OVERHANG` of its side.
      const inset = side / 2 - Math.round(side * OVERHANG);
      let best = { x: 0, y: 0, room: -1 };
      for (let attempt = 0; attempt < TRIES; attempt++) {
        const x = inset + rand.next() * (width - 2 * inset);
        const y = inset + rand.next() * (height - 2 * inset);
        // How many half-sides of the two pictures fit between the middles: 1 is touching, under 1 is overlapping. The tightest neighbor decides.
        const room = placed.reduce((least, other) => Math.min(least, Math.hypot(x - other.x, y - other.y) / ((side + other.side) / 2)), Infinity);
        if (room > best.room) best = { x, y, room };
      }
      placed.push({ x: best.x, y: best.y, side });
    }
    return placed.map(({ x, y, side }, index) => ({
      image: pictures[index].image,
      x: Math.round(x - side / 2),
      y: Math.round(y - side / 2),
      width: side,
      height: side,
      keepAspect,
    }));
  },
};
