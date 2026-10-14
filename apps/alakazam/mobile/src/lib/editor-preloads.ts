import { fitted } from '@/src/lib/editor-pictures';
import type { AbraImage, AbraImageLike } from '@alakazam/mobile';
import type { Image as AbraPixels } from '@abra/core/image';
import { TransformAlgorithm, type MaskLike } from '@abra/core/mask';
import type { DataLike, Preloaded, SliderControl } from '@/src/lib/edit-sections';

// This module holds native images, so it imports nothing but types from the controls: saving a control or a tool re-runs
// the controls and everything that imports them, and what is held here must survive that.

type PreloadFunction = NonNullable<SliderControl['preload']>;

/**
 * What each preload function made, for each photo size it has been asked for. Controls that use the same function share its
 * result, so a group such as Makeup runs face analysis once and hands every control the masks it needs. Every entry is its own
 * set of images (a size made from another is a resized copy), so each is freed once.
 */
const preloads = new Map<PreloadFunction, Map<number, Preloaded>>();
/**
 * What each preload function made when it ran on the photo, or was given (see `provideAnalysis`). It is made from the photo and
 * not from the edits, so it is kept when the edit base changes (a healed copy of the same photo) and freed with the session.
 */
const analyses = new Map<PreloadFunction, Preloaded>();
/** Which preload function a control last used at a width, so Fast Refresh can retire an obsolete function promptly. */
const controlPreloads = new Map<string, PreloadFunction>();

/**
 * The widest photo a preload is run on. The models look at a fixed, small picture whatever the size of the photo (a face is
 * cropped and shrunk for them), so running them on more pixels only costs time, and a full-size photo takes minutes.
 */
const ANALYSIS_WIDTH = 1280;

/**
 * Things made from a preloaded set of masks that only change when one particular setting does (the region around an eye, which
 * only its fullness changes; a lip edge, which only its softness changes). A tool asks for one by a `group` and the `variant` of the
 * setting, and moving any other slider gets the one made before. Each group holds only the latest variant, so dragging the setting
 * does not pile up masks. They belong to the set they were made from, and are freed with it.
 */
const derived = new Map<Preloaded, Map<string, { variant: string; data: DataLike | undefined }>>();

const values = (held: Preloaded): DataLike[] => (Array.isArray(held) ? held : Object.values(held));

const free = (held: Preloaded) => {
  for (const data of values(held)) (data as AbraPixels).uniffiDestroy();
  for (const { data } of derived.get(held)?.values() ?? []) (data as AbraPixels | undefined)?.uniffiDestroy();
  derived.delete(held);
};

/**
 * What `make` makes for the `group` of `owner` (the preloaded set a tool was given) at this `variant` of its setting. It is made
 * once and kept, so a tool that runs on every move of a slider does the work only when the setting that changes it moves. What
 * `make` returns is owned by this, and a tool must not free it; nothing is a result too, and is kept as one.
 */
export const derivedFor = (
  owner: Preloaded,
  group: string,
  variant: string,
  make: () => DataLike | undefined,
): DataLike | undefined => {
  let groups = derived.get(owner);
  if (!groups) derived.set(owner, (groups = new Map()));
  const held = groups.get(group);
  if (held?.variant === variant) return held.data;
  (held?.data as AbraPixels | undefined)?.uniffiDestroy();
  const data = make();
  groups.set(group, { variant, data });
  return data;
};

/**
 * A copy of `held` for a `width` x `height` photo, or nothing when it holds something that cannot be resized. A preload that is
 * run once and wanted at other sizes (the editor shows a shrunk photo and saves the full-size one) holds masks, which can be:
 * the same mask at another size, with no model run. What it names its masks by must not depend on the size.
 */
const resizedTo = (held: Preloaded, width: number, height: number): Preloaded | undefined => {
  if (!values(held).every(data => 'resized' in data)) return undefined;
  const resize = (data: DataLike) => (data as MaskLike).resized(width, height, TransformAlgorithm.Bilinear) as DataLike;
  return Array.isArray(held)
    ? held.map(resize)
    : Object.fromEntries(Object.entries(held).map(([name, data]) => [name, resize(data)]));
};

/**
 * What the control's `preload` made for a photo `width` pixels wide. The function is run once for a photo, on a copy of the
 * `source` photo that is no wider than `ANALYSIS_WIDTH`, and what it made is kept; every other size is that resized, which is
 * quick. Without a `source`, a size that is not held and cannot be made from one that is gives nothing.
 */
export const preloadFor = (control: SliderControl, width: number, source?: AbraImageLike): Preloaded | undefined => {
  const preload = control.preload;
  if (!preload) return undefined;
  const controlKey = `${control.key}:${width}`;
  const previous = controlPreloads.get(controlKey);
  if (previous && previous !== preload) {
    controlPreloads.delete(controlKey);
    const stillUsed = [...controlPreloads.values()].includes(previous);
    if (!stillUsed) {
      for (const held of preloads.get(previous)?.values() ?? []) free(held);
      preloads.delete(previous);
    }
  }
  controlPreloads.set(controlKey, preload);
  let byWidth = preloads.get(preload);
  // What it made for this photo before, or what it was given, is the start of every size.
  const provided = analyses.get(preload);
  if (provided && !byWidth) {
    preloads.set(preload, (byWidth = new Map([[analysisWidth(provided), provided]])));
  }
  let held = byWidth?.get(width);

  // Made before, at another size: this is that, resized. From the smallest copy that is as wide as this one, so a small size is
  // not made by shrinking the largest, which costs a pass over every pixel of it; or from the widest when none is.
  if (!held && byWidth?.size) {
    const sizes = [...byWidth.entries()].sort(([a], [b]) => a - b);
    const [otherWidth, other] = sizes.find(([otherWidth]) => otherWidth >= width) ?? sizes[sizes.length - 1];
    const sample = values(other)[0] as unknown as MaskLike | undefined;
    const height = source ? source.height() : sample ? Math.round((sample.height() * width) / otherWidth) : 0;
    const resizeStarted = Date.now();
    if (height > 0) held = resizedTo(other, width, height);
    if (held) byWidth.set(width, held);
    if (__DEV__) console.debug(`[preload] ${control.key} ${width}px from ${otherWidth}px, ms:`, Date.now() - resizeStarted);
  }

  // Not made yet: run it, once, on a photo that is not too large for it.
  if (!held && source) {
    // A copy, shrunk when it is wider than the analysis needs. Both are the tool's to change.
    const photo = fitted(source as AbraImage, ANALYSIS_WIDTH, ANALYSIS_WIDTH * 4) as AbraImageLike;
    const analysedWidth = photo.width();
    const analysisStarted = Date.now();
    try {
      const made = preload(photo);
      if (__DEV__) console.debug(`[preload] ${control.key} analysed at ${analysedWidth}px, ms:`, Date.now() - analysisStarted);
      // One image or mask has methods to free it; an object of them does not.
      let analysis: Preloaded;
      if (!made) analysis = [];
      else if (Array.isArray(made)) analysis = made;
      else if ('uniffiDestroy' in made) analysis = [made as unknown as DataLike];
      else analysis = made as Record<string, DataLike>;
      if (!byWidth) preloads.set(preload, (byWidth = new Map()));
      byWidth.set(analysedWidth, analysis);
      if (values(analysis).length > 0) analyses.set(preload, analysis);
      held = analysis;
      if (analysedWidth !== width) {
        // What was made is held at its own size; the size asked for is a resized copy, or the same thing when it cannot be.
        const resized = resizedTo(analysis, width, source.height());
        if (resized) byWidth.set(width, (held = resized));
      }
    } finally {
      // They are native images the garbage collector cannot see the size of.
      (photo as AbraPixels).uniffiDestroy();
    }
  }
  return held;
};

/** The width of the photo a set of masks was made from. */
const analysisWidth = (analysis: Preloaded): number => (values(analysis)[0] as unknown as MaskLike).width();

/**
 * What `preload` made when it ran on the photo, or nothing if it has not. It can be saved with the project and given back with
 * `provideAnalysis`, so opening the project does not run the models again.
 */
export const analysisOf = (preload: PreloadFunction): Preloaded | undefined => analyses.get(preload);

/** Gives `preload` what it would make for the photo (not empty), as if it had run, so it never runs. This takes the masks. */
export const provideAnalysis = (preload: PreloadFunction, analysis: Preloaded) => {
  const previous = analyses.get(preload);
  if (previous && previous !== analysis) free(previous);
  analyses.set(preload, analysis);
};

/** Frees every preloaded image made at a size, when the edit base changes. What the preloads made of the photo is kept. */
export const clearPreloads = () => {
  const kept = new Set(analyses.values());
  for (const byWidth of preloads.values()) for (const held of byWidth.values()) if (!kept.has(held)) free(held);
  preloads.clear();
  controlPreloads.clear();
};

/** Frees every preloaded image and everything the preloads made of the photo, when the photo goes away. */
export const releasePreloads = () => {
  clearPreloads();
  for (const analysis of analyses.values()) free(analysis);
  analyses.clear();
};
