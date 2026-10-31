import { type Rand } from '@alakazam/sdk';

/** A whole number from `min` to `max`, either of which it can be. */
export const between = (rand: Rand, min: number, max: number) => min + Math.floor(rand.next() * (max - min + 1));

/** The numbers 0 up to `count` in a random order. */
export const shuffled = (count: number, rand: Rand) => {
  const order = Array.from({ length: count }, (_, index) => index);
  for (let at = count - 1; at > 0; at--) {
    const other = between(rand, 0, at);
    [order[at], order[other]] = [order[other], order[at]];
  }
  return order;
};

/**
 * Which picture goes in each of `cells` cells: any picture that has not been used yet, and once every picture has been used, any of
 * them again. So there can be more cells than pictures, or fewer.
 */
export const dealt = (cells: number, images: number, rand: Rand) => {
  const chosen: number[] = [];
  let unused: number[] = [];
  for (let cell = 0; cell < cells && images > 0; cell++) {
    if (unused.length === 0) unused = shuffled(images, rand);
    chosen.push(unused.pop() as number);
  }
  return chosen;
};
