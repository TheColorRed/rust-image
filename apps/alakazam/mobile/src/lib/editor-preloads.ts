import type { AbraImageLike } from '@alakazam/mobile';
import type { Image as AbraPixels } from '@abra/core/image';
import type { DataLike, Preloaded, SliderControl } from '@/src/lib/edit-sections';

// This module holds native images, so it imports nothing but types from the controls: saving a control or a tool re-runs
// the controls and everything that imports them, and what is held here must survive that.

/**
 * What each control's `preload` made, for the photo size it was made at: `control key:width` to what it returned, and the
 * function that made it. A saved edit to the function gives the control a new one, and what the old one made is no longer
 * what the control would make, so it is thrown away and made again.
 */
const preloads = new Map<string, { preload: SliderControl['preload']; held: Preloaded }>();

const free = (held: Preloaded) => {
  for (const data of Array.isArray(held) ? held : Object.values(held)) (data as AbraPixels).uniffiDestroy();
};

/**
 * What the control's `preload` made for a photo `width` pixels wide. When it has not been made at that width yet and a `source`
 * photo is given, it is made from a copy of the source now and kept, so the next call is free. Without a source, a miss gives
 * nothing.
 */
export const preloadFor = (control: SliderControl, width: number, source?: AbraImageLike): Preloaded | undefined => {
  if (!control.preload) return undefined;
  const key = `${control.key}:${width}`;
  const entry = preloads.get(key);
  if (entry && entry.preload !== control.preload) {
    free(entry.held);
    preloads.delete(key);
  }
  let held = entry?.preload === control.preload ? entry.held : undefined;
  if (!held && source) {
    const photo = source.toImage();
    try {
      const made = control.preload(photo);
      // One image or mask has methods to free it; an object of them does not.
      if (!made) held = [];
      else if (Array.isArray(made)) held = made;
      else if ('uniffiDestroy' in made) held = [made as unknown as DataLike];
      else held = made as Record<string, DataLike>;
      preloads.set(key, { preload: control.preload, held });
    } finally {
      // They are native images the garbage collector cannot see the size of.
      (photo as AbraPixels).uniffiDestroy();
    }
  }
  return held;
};

/** Frees every preloaded image, when the photo they were made from goes away. */
export const clearPreloads = () => {
  for (const { held } of preloads.values()) free(held);
  preloads.clear();
};
