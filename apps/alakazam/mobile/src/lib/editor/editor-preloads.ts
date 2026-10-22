import { fitted, maskInEditFrame } from '@/src/lib/editor/editor-pictures';
import type { AbraImage, AbraImageLike } from '@alakazam/mobile';
import type { Image as AbraPixels } from '@abra/core/image';
import type { MaskLike } from '@abra/core/mask';
import { originalImage } from '@/src/state/session';
import type { DataLike, Preloaded, Preloading, SliderControl } from '@/src/lib/project/edit-sections';
import { analysisVersion } from '@/src/state/analysis';
import { isHandle, release } from '@/src/lib/native/native-handles';

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
/** The background preloads (see `SliderControl.preload`) that have started and not finished, so each is started once for a photo. */
const running = new Set<PreloadFunction>();
/**
 * How many background preloads run at once. Each loads several models, and the device runs out of memory when they all load together.
 */
const MAX_BACKGROUND_PRELOADS = 2;
/** Preloads waiting for a free slot in `running`; each asks for its control's preload again when it is called. */
const waiting: Array<() => void> = [];
const startWaiting = () => {
  while (waiting.length && running.size < MAX_BACKGROUND_PRELOADS) waiting.shift()?.();
};
/** Counts the photos let go of, so a background preload that finishes after its photo is gone frees what it made. */
let session = 0;

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
  for (const data of values(held)) release(data);
  for (const { data } of derived.get(held)?.values() ?? []) release(data);
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
  release(held?.data);
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
  // The masks were found in the uncropped original, so they are cut by the applied crops on the way.
  const resize = (data: DataLike) => maskInEditFrame(data as MaskLike, width, height) as DataLike;
  return Array.isArray(held)
    ? held.map(resize)
    : Object.fromEntries(Object.entries(held).map(([name, data]) => [name, resize(data)]));
};

/**
 * What the control's `preload` made for a photo of this size (the photo itself, or anything with its width and height). The function is
 * run once for a photo, on a copy of the `source` photo that is no wider than `ANALYSIS_WIDTH`, and what it made is kept; every other
 * size is that resized to exactly this width and height, which is quick, so a mask always fits the photo it is used on. Without a
 * `source`, a size that is not held and cannot be made from one that is gives nothing.
 */
export const preloadFor = (
  control: SliderControl,
  size: { width(): number; height(): number },
  source?: AbraImageLike,
): Preloaded | undefined => {
  const preload = control.preload;
  if (!preload) return undefined;
  // Read now: the photo can be let go of before a preload that is waiting for a free slot runs.
  const width = size.width();
  const height = size.height();
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
  // It ran and made nothing (no face), which is the same at every size.
  if (provided && values(provided).length === 0) return provided;
  let held = byWidth?.get(width);

  // Made before, in the original: this is that, cut by the crops and resized. Masks are always made this way (even at the size
  // they were found at, which a crop changes), and anything that cannot be resized is handed out as it was.
  if (!held && provided) {
    const resizeStarted = Date.now();
    held = resizedTo(provided, width, height);
    if (held) {
      if (!byWidth) preloads.set(preload, (byWidth = new Map()));
      byWidth.set(width, held);
    } else if (!byWidth?.size) {
      preloads.set(preload, (byWidth = new Map([[width, provided]])));
      held = provided;
    }
    if (__DEV__) console.debug(`[preload] ${control.key} ${width}px from the analysis, ms:`, Date.now() - resizeStarted);
  }

  // Not made yet: run it, once, on a photo that is not too large for it. One that is running in the background is waited for.
  if (!held && source && !running.has(preload)) {
    if (running.size >= MAX_BACKGROUND_PRELOADS) {
      waiting.push(() => preloadFor(control, { width: () => width, height: () => height }, source));
      return undefined;
    }
    // A copy, shrunk when it is wider than the analysis needs. Both are the tool's to change. It is of the original even when the
    // edit base is cropped: the masks are cut by the crops when they are used, so they must all be found in the same frame.
    const photo = fitted((originalImage.value ?? source) as AbraImage, ANALYSIS_WIDTH, ANALYSIS_WIDTH * 4) as AbraImageLike;
    const analysedWidth = photo.width();
    const analysisStarted = Date.now();
    try {
      const made = preload(photo);
      if (made instanceof Promise) {
        // It runs in the background: the editor goes on without it, and draws again when it is made.
        running.add(preload);
        const started = session;
        made
          .then(result => {
            running.delete(preload);
            // Nothing means it could not run (a missing model), and that is not an answer to keep: it is tried again later.
            if (!result) return;
            const analysis = asPreloaded(result);
            if (started !== session) return free(analysis);
            provideAnalysis(preload, analysis);
            analysisVersion.next(analysisVersion.value + 1);
          })
          .catch(error => {
            running.delete(preload);
            console.warn(`[preload] ${control.key} failed:`, error);
          })
          .finally(startWaiting);
        if (__DEV__) console.debug(`[preload] ${control.key} started in the background`);
        return undefined;
      }
      if (__DEV__) {
        const count = !made ? 0 : Array.isArray(made) ? made.length : isHandle(made) ? 1 : Object.keys(made).length;
        console.debug(`[preload] ${control.key} analysed at ${analysedWidth}px, ms:`, Date.now() - analysisStarted, 'found:', count);
      }
      const analysis = asPreloaded(made);
      if (!byWidth) preloads.set(preload, (byWidth = new Map()));
      // A preload that gave nothing back could not run (a missing model), so it is run again later. One that gave an empty set ran.
      if (made) analyses.set(preload, analysis);
      // What was made is of the original; the size asked for is that cut by the crops and resized, or the same thing when it cannot be.
      held = resizedTo(analysis, width, height) ?? analysis;
      byWidth.set(width, held);
    } finally {
      // They are native images the garbage collector cannot see the size of.
      release(photo);
    }
  }
  return held;
};

/**
 * Runs `preload` on the photo and keeps what it makes, like a control that needs it would, and gives back when it is done. It does
 * nothing for one that is made or running already. It is for building the analyses one after another, because their models are large
 * and the device runs out of memory when several are loaded at once.
 */
export const buildAnalysis = async (preload: PreloadFunction, source: AbraImageLike): Promise<void> => {
  if (analyses.has(preload) || running.has(preload)) return;
  const photo = fitted(source as AbraImage, ANALYSIS_WIDTH, ANALYSIS_WIDTH * 4) as AbraImageLike;
  running.add(preload);
  const started = session;
  try {
    const result = await preload(photo);
    // Nothing means it could not run (a missing model), and that is not an answer to keep.
    if (!result) return;
    const analysis = asPreloaded(result);
    if (started !== session) return free(analysis);
    provideAnalysis(preload, analysis);
    analysisVersion.next(analysisVersion.value + 1);
  } finally {
    running.delete(preload);
    release(photo);
    startWaiting();
  }
};

/** What a preload gave back, as the set the tools get: an empty one when it gave nothing. */
const asPreloaded = (made: Preloading): Preloaded => {
  // One image or mask has methods to free it; an object of them does not.
  if (!made) return [];
  if (Array.isArray(made)) return made;
  return isHandle(made) ? [made as unknown as DataLike] : (made as Record<string, DataLike>);
};

/**
 * What `preload` made when it ran on the photo, or nothing if it has not. It can be saved with the project and given back with
 * `provideAnalysis`, so opening the project does not run the models again.
 */
export const analysisOf = (preload: PreloadFunction): Preloaded | undefined => analyses.get(preload);

/** Gives `preload` what it would make for the photo (an empty set when it found nothing), as if it had run, so it never runs. This takes the masks. */
export const provideAnalysis = (preload: PreloadFunction, analysis: Preloaded) => {
  const previous = analyses.get(preload);
  if (previous && previous !== analysis) free(previous);
  analyses.set(preload, analysis);
  // What was held for a size before this is not what it would be now.
  for (const held of preloads.get(preload)?.values() ?? []) if (held !== previous && held !== analysis) free(held);
  preloads.delete(preload);
};

/**
 * Forgets what `preload` made for the photo, so it runs again the next time a control needs it. It does nothing while the preload is
 * still running, and the caller must have checked that the preload can run (its models are installed), or what it made is lost for good.
 */
export const forgetAnalysis = (preload: PreloadFunction) => {
  if (running.has(preload)) return;
  const previous = analyses.get(preload);
  if (previous) free(previous);
  analyses.delete(preload);
  for (const held of preloads.get(preload)?.values() ?? []) if (held !== previous) free(held);
  preloads.delete(preload);
};

/** Frees every preloaded image made at a size, when the edit base changes. What the preloads made of the photo is kept. */
export const clearPreloads = () => {
  const kept = new Set(analyses.values());
  for (const byWidth of preloads.values()) for (const held of byWidth.values()) if (!kept.has(held)) free(held);
  preloads.clear();
  controlPreloads.clear();
  waiting.length = 0;
};

/** Frees every preloaded image and everything the preloads made of the photo, when the photo goes away. */
export const releasePreloads = () => {
  session++;
  running.clear();
  waiting.length = 0;
  clearPreloads();
  for (const analysis of analyses.values()) free(analysis);
  analyses.clear();
};
