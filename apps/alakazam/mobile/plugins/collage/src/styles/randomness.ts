import { shuffled, type Rand } from '@alakazam/sdk';

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
