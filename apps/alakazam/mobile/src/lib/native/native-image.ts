import { AbraImage, type AbraImageLike } from '@alakazam/mobile';

/** Generated APIs return the interface; editor-owned handles also need explicit disposal. */
export function ownedImage(image: AbraImageLike): AbraImage {
  if (image instanceof AbraImage) return image;
  throw new Error('Native image API returned a handle without ownership');
}
