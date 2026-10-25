import { devDebug, warn, error } from '@/src/lib/log';
import { personIn } from '@/src/lib/ai/analysis';
import { batch } from '@/src/lib/utils/batch';
import {
  applyAction,
  applyJoint,
  applySlider,
  areaPoints,
  EDIT_SECTIONS,
  findSection,
  layerSheet,
  controlsWith,
  leafControls,
  isSliderApplied,
  scaleBetween,
  sliderContext,
  sliderDefault,
  sliderValues,
  thumbnailControls,
  type AdjustmentValue,
  type ActionControl,
  type EditSection,
  type PickerControl,
  type SliderControl,
} from '@/src/lib/project/edit-sections';
import { fitted, releasePictures, setShown, shrunkOf } from '@/src/lib/editor/editor-pictures';
import { withLayers, type Sheet } from '@/src/lib/beauty/layers';
import { clearPreloads, preloadFor, releasePreloads } from '@/src/lib/editor/editor-preloads';
import { buildControlThumbnails, thumbnailPixels } from '@/src/lib/editor/editor-thumbnails';
import { type ProjectRecipe } from '@/src/lib/project/project-store';
import { adjustmentKey, isPersonAdjustment, personAdjustmentTargets } from '@/src/lib/beauty/skin-adjustments';
import {
  activeSectionKey,
  activeSectionKey$,
  adjustments,
  adjustments$,
  appliedActions,
  appliedActions$,
  focusedControlKey,
  focusedControlKey$,
  historyVersion$,
} from '@/src/state/edits';
import { analysisVersion$ } from '@/src/state/analysis';
import { previewBox, previewBox$ } from '@/src/state/gestures';
import { personSelection } from '@/src/state/person-selection';
import { controlThumbnails } from '@/src/state/preview';
import { busy, currentImage, editBaseImage, ready$, replayHidden } from '@/src/state/session';
import { type Image as AbraPixels } from '@abra/core/image';
import { AbraImage, type AbraImageLike, type ThumbnailPreview } from '@alakazam/mobile';
import { combineLatest, Observable, type Subscription } from 'rxjs';
import { debounceTime, distinctUntilChanged, filter, startWith, switchMap, tap } from 'rxjs/operators';
import { building, release } from '@/src/lib/native/native-handles';

// #region: The full-size picture, which is not shown

/** Whether the edit has changed since the full-size picture (`currentImage`) was made, so it is out of date. */
let fullReplayPending = false;
let currentImageRecipe = '';
let currentImageBase: AbraImage | null = null;

const recipeSignature = (recipe: ProjectRecipe) => JSON.stringify(recipe);

/** Marks the already-saved full-size image as the result of this recipe when a project opens. */
export function markCurrentImageRecipe(recipe: ProjectRecipe, base = editBaseImage.value) {
  currentImageRecipe = recipeSignature(recipe);
  currentImageBase = base;
}

/**
 * Makes the edit on the full-size photo. It is not shown: it is `currentImage`, for what needs every pixel. It takes seconds on
 * a large photo and holds the JavaScript thread while it does, so it is only made when something asks for it
 * (`flushFullReplay`), never behind the user's back while they edit.
 */
function runFullReplay() {
  fullReplayPending = false;
  const base = editBaseImage.value;
  if (!base) return;
  const started = Date.now();
  const recipe: ProjectRecipe = { adjustments: adjustments.value, appliedActions: appliedActions.value };
  const next = renderRecipe(base, recipe);
  release(currentImage.value);
  currentImage.next(next);
  currentImageRecipe = recipeSignature(recipe);
  currentImageBase = base;
  devDebug('edit', 'full-size replay ms (not shown):', Date.now() - started);
}

/**
 * Makes the full-size edit now if the one there is out of date. Whatever needs the full-size photo, such as saving or the
 * blemish tool, calls this first, so it works on the photo as it is edited now and not on the one before the last change.
 */
export function flushFullReplay() {
  const recipe: ProjectRecipe = { adjustments: adjustments.value, appliedActions: appliedActions.value };
  // The editor can become ready before its first preview replay has populated `currentImage`.
  // Pixel tools (blemish and object removal) may be used during that gap, so make sure they have
  // a full-size image for the current recipe instead of silently doing nothing or using a stale one.
  if (
    fullReplayPending ||
    !currentImage.value ||
    currentImageRecipe !== recipeSignature(recipe) ||
    currentImageBase !== editBaseImage.value
  )
    runFullReplay();
}

/** Lets go of every screen-size picture and forgets a full-size edit that was waiting, when the session ends. */
export function releaseShownImages() {
  fullReplayPending = false;
  currentImageRecipe = '';
  currentImageBase = null;
  clearRenderCache();
  releasePreloads();
  preloadedBase = null;
  replayedBase = null;
  releasePictures();
}

// #endregion

/** Gap between an edit and its replay, so the tap's immediate UI feedback can be drawn first. */
const REPLAY_DELAY_MS = 60;
// The preview box eases between heights over 240 ms when the editor panel changes size. Replaying
// the whole edit stack on every animation frame turns that one layout change into a burst of
// synchronous native renders, starving presses and slider work on the JS thread.
const PREVIEW_BOX_TWEEN_MS = 240;
const REPLAY_BOX_SETTLE_MS = PREVIEW_BOX_TWEEN_MS + 40;

const findAction = (key: string): { control: ActionControl; section: EditSection } | undefined => {
  for (const section of EDIT_SECTIONS) {
    for (const control of leafControls(section.controls)) {
      if (control.kind === 'action' && control.key === key) return { control, section };
    }
  }
  return undefined;
};

/**
 * Where in the stack a step runs. The beauty fixes come first, then the makeup, which is laid on the photo as layers of its own
 * (the canvas has the photo as its `base` layer and a layer for each makeup control), then the look (the moods and the
 * adjustments), so the look covers the makeup too, and then the rest. Steps keep their order within a stage.
 */
const stageOf = (sectionKey: string, layered: boolean): number => {
  if (layered) return 1;
  if (sectionKey === 'section-beauty') return 0;
  return sectionKey === 'section-colorization' || sectionKey === 'section-adjustments' ? 2 : 3;
};

/** One applied control: how to run it on an image. */
type Step = {
  key: string;
  /** The slider's value; actions have none. */
  value?: AdjustmentValue;
  /** Where in the stack it runs: see `stageOf`. */
  stage: number;
  /** Whether it only makes the photo remember library effects for the GPU (see `SliderControl.deferred`), so it costs nothing at full size. */
  deferred: boolean;
  /** Identifies what the step does, so a rendered stack can be reused up to the first step that differs. */
  signature: string;
  run: (image: AbraImageLike, scale: number) => void;
  /** For a control that has a layer: the layer's sheet, so that the steps next to it go on one canvas. `run` does it alone. */
  sheet?: (image: AbraImageLike, scale: number) => Sheet;
};

/**
 * The applied controls in the order they run: one-shot actions, then sliders in section order.
 * `excludeGroups` leaves out applied actions of those mutually exclusive groups. `slider` is the slider being dragged:
 * without a `value` it is left out, and with one it is included at that value whether or not it is applied.
 * `pending` is an action about to be applied: it goes after the applied actions, where toggling it will put it.
 * `recipe` is the edit state to run; it is the live one unless a saved project's step is being replayed.
 */
function appliedSteps(
  excludeGroups?: ReadonlySet<string>,
  slider?: { key: string; value?: AdjustmentValue },
  pending?: ActionControl,
  recipe: ProjectRecipe = { adjustments: adjustments.value, appliedActions: appliedActions.value },
): Step[] {
  const steps: Step[] = [];
  const keys =
    pending && (pending.repeatable || !recipe.appliedActions.includes(pending.key))
      ? [...recipe.appliedActions, pending.key]
      : recipe.appliedActions;
  for (const key of keys) {
    const found = findAction(key);
    if (!found) continue;
    const { control, section } = found;
    if (!control.apply && !control.tool) continue;
    // The action about to be applied replaces the others in its group, so it is never left out with them.
    if (control !== pending && control.group && excludeGroups?.has(control.group)) continue;
    steps.push({
      key,
      stage: stageOf(section.key, false),
      deferred: !!control.deferred && !control.apply,
      signature: JSON.stringify(['action', key]),
      run: (image, scale) => applyAction(control, image, scale),
    });
  }
  const jointsDone = new Set<string>();
  for (const section of EDIT_SECTIONS) {
    for (const control of leafControls(section.controls)) {
      if (control.kind !== 'slider') continue;
      const { joint } = control;
      if (joint) {
        // The sliders of a mixed control with one tool are one edit, made once at the first of them.
        if (jointsDone.has(joint.key)) continue;
        jointsDone.add(joint.key);
        const committedOf = (each: SliderControl) =>
          slider?.key === each.key ? slider.value : recipe.adjustments[each.key];
        const inJoint = joint.sliders.some(each => each.key === slider?.key);
        if (inJoint && slider?.value === undefined) continue;
        if (!joint.sliders.some(each => each.key === slider?.key || isSliderApplied(each, committedOf(each)))) continue;
        const values = joint.sliders.map(each => sliderValues(each, committedOf(each)));
        steps.push({
          key: control.key,
          stage: stageOf(section.key, false),
          deferred: false,
          signature: JSON.stringify([joint.key, values]),
          run: (image, scale) => applyJoint(joint, image, values, scale),
        });
        continue;
      }
      if (isPersonAdjustment(control.key)) {
        if (!control.tool && !control.layer) throw new Error(`Person control has no effect definition: ${control.key}`);
        const activeKey = adjustmentKey(control.key, personSelection.value.selectedId);
        const targets = personAdjustmentTargets(recipe.adjustments, control.key);
        // The people with a value of their own, who the layer for everyone leaves out.
        const own = targets.flatMap(target => (target.personId === null ? [] : [target.personId]));
        for (const target of targets) {
          if (slider?.key === control.key && target.key === activeKey) continue;
          if (!isSliderApplied(control, target.value)) continue;
          // The box of the person, in the pixels of the image it is run on.
          const personOn = (image: AbraImageLike) => personIn(image.width(), target.personId);
          steps.push({
            key: target.key,
            value: target.value,
            stage: stageOf(section.key, !!control.layer),
            deferred: false,
            signature: JSON.stringify([target.key, target.value, target.personId]),
            sheet: control.layer
              ? (image, scale) =>
                  layerSheet(
                    control,
                    sliderValues(control, target.value),
                    sliderContext(control, image, scale, personOn(image), target.personId === null ? own : undefined),
                    target.key,
                  )
              : undefined,
            run: (image, scale) => {
              // A tool is told where the person is, in the pixels of the image it is run on.
              applySlider(control, image, target.value, scale, personOn(image));
            },
          });
        }
        continue;
      }
      const dragged = slider?.key === control.key;
      if (dragged && slider.value === undefined) continue;
      const committed = dragged ? slider.value : recipe.adjustments[control.key];
      const areas = control.areaKeys?.map(key => areaPoints(recipe.adjustments[key]));
      // A color with an area drawn still applies before its slider is moved.
      const hasArea = !!areas?.some(rings => (rings ?? []).some(ring => ring.length > 0));

      if (!dragged && !hasArea && !isSliderApplied(control, committed)) continue;
      const value = committed ?? sliderDefault(control);
      steps.push({
        key: control.key,
        value,
        stage: stageOf(section.key, !!control.layer),
        deferred: !!control.deferred && !control.layer,
        signature: JSON.stringify([control.key, value, areas]),
        sheet: control.layer
          ? (image, scale) => layerSheet(control, sliderValues(control, value), sliderContext(control, image, scale))
          : undefined,
        run: (image, scale) => applySlider(control, image, value, scale, undefined, areas),
      });
    }
  }
  // Array.sort keeps the order of steps with the same stage.
  return steps.sort((first, second) => first.stage - second.stage);
}

/** Renders a saved recipe onto a fresh copy of `base`, without touching the live edit state. The caller owns the result. */
export function renderRecipe(base: AbraImage, recipe: ProjectRecipe): AbraImage {
  return building(
    () => base.clone() as AbraImage,
    next => runSteps(next, appliedSteps(undefined, undefined, undefined, recipe), base),
  );
}

/** The stable control identifiers that define a saved render-cache stack. */
export const stackSignatures = (recipe: ProjectRecipe) =>
  appliedSteps(undefined, undefined, undefined, recipe).map(step => step.signature);

/**
 * The last stack rendered for the live edit, kept so a change only runs the steps from the first one that differs. `snaps[i]` is
 * the picture before step `i`, and `snaps[steps.length]` is the finished picture.
 */
type RenderCache = {
  base: number;
  full: boolean;
  width: number;
  height: number;
  signatures: string[];
  snaps: (AbraImage | undefined)[];
};
let renderCache: Record<string, RenderCache | undefined> = {};

const cacheKey = (full: boolean, variant = 'main') => `${full ? 'full' : 'screen'}:${variant}`;

const clearRenderCacheFor = (full: boolean, variant?: string) => {
  const prefix = `${full ? 'full' : 'screen'}:`;
  const keys = variant ? [cacheKey(full, variant)] : Object.keys(renderCache).filter(key => key.startsWith(prefix));
  for (const key of keys) {
    renderCache[key]?.snaps.forEach(snap => release(snap));
    delete renderCache[key];
  }
};

/** Frees the pictures the rendered stack kept, so the next render starts from the base. */
function clearRenderCache() {
  clearRenderCacheFor(false);
  clearRenderCacheFor(true);
}

/** Restores the screen-size control snapshots saved with a project, without invoking the control tools. */
export function restoreProjectRenderCache(
  base: AbraImage,
  recipe: ProjectRecipe,
  layers: { signature: string; image: AbraImage }[],
) {
  clearRenderCacheFor(false);
  try {
    const steps = appliedSteps(undefined, undefined, undefined, recipe);
    if (layers.length !== steps.length || layers.some((layer, index) => layer.signature !== steps[index]?.signature)) {
      throw new Error(`Project layer cache does not match its ${steps.length} saved controls.`);
    }
    const start = shrunkOf('base', base);
    const snaps: (AbraImage | undefined)[] = [start.clone() as AbraImage];
    try {
      for (const layer of layers) snaps.push(fitted(layer.image, start.width(), start.height()));
      renderCache[cacheKey(false)] = {
        base: imageId(base),
        full: false,
        width: start.width(),
        height: start.height(),
        signatures: steps.map(step => step.signature),
        snaps,
      };
    } catch (error) {
      snaps.forEach(release);
      throw error;
    }
  } finally {
    layers.forEach(layer => release(layer.image));
  }
}

/** Writes the currently rendered full-resolution composite after each control to its project layer file. */
export function writeProjectRenderLayers(
  base: AbraImage,
  recipe: ProjectRecipe,
  write: (index: number, signature: string, image: AbraImage) => string,
) {
  const steps = appliedSteps(undefined, undefined, undefined, recipe);
  const files: { signature: string; file: string }[] = [];
  const image = base.clone() as AbraImage;
  try {
    runSteps(image, steps, base, 0, index => {
      if (index === 0) return;
      const step = steps[index - 1]!;
      files.push({ signature: step.signature, file: write(index - 1, step.signature, image) });
    });
    return { layers: files, image };
  } catch (error) {
    release(image);
    throw error;
  } finally {
    // A deferred full-size preview may have kept runtime snapshots. Durable copies now live in the project folder.
    clearRenderCacheFor(true);
  }
}

/**
 * Renders the live edit onto a fresh copy of the edit base, as `renderEditStack` does, but reuses what the last render made up to
 * the first step that changed. The caller owns the result.
 */
function renderCachedStack(base: AbraImage, full: boolean, steps = appliedSteps(), variant = 'main'): AbraImage {
  const start = full ? base : shrunkOf('base', base);
  const signatures = steps.map(step => step.signature);
  const baseId = imageId(base);
  const width = start.width();
  const height = start.height();
  let image: AbraImage;
  let from = 0;
  const key = cacheKey(full, variant);
  const kept = renderCache[key];
  const main = variant === 'main' ? undefined : renderCache[cacheKey(full)];
  if (
    kept &&
    kept.base === baseId &&
    kept.full === full &&
    (!full || (kept.width === width && kept.height === height))
  ) {
    let firstChanged = 0;
    while (
      firstChanged < signatures.length &&
      firstChanged < kept.signatures.length &&
      signatures[firstChanged] === kept.signatures[firstChanged]
    ) {
      firstChanged++;
    }
    if (firstChanged === signatures.length && firstChanged === kept.signatures.length) {
      return kept.snaps[firstChanged]!.clone() as AbraImage;
    }
    from = firstChanged;
    while (!kept.snaps[from]) from--;
    kept.snaps.forEach((snap, index) => {
      if (index > from) {
        release(snap);
        kept.snaps[index] = undefined;
      }
    });
    kept.signatures = signatures;
    image = kept.snaps[from]!.clone() as AbraImage;
  } else if (
    main &&
    main.base === baseId &&
    main.full === full &&
    (!full || (main.width === width && main.height === height))
  ) {
    let prefix = 0;
    while (
      prefix < signatures.length &&
      prefix < main.signatures.length &&
      signatures[prefix] === main.signatures[prefix]
    )
      prefix++;
    clearRenderCacheFor(full, variant);
    const snaps = main.snaps.slice(0, prefix + 1).map(snap => snap?.clone() as AbraImage | undefined);
    renderCache[key] = { base: baseId, full, width: main.width, height: main.height, signatures, snaps };
    from = prefix;
    image = snaps[prefix]!.clone() as AbraImage;
  } else {
    clearRenderCacheFor(full, variant);
    renderCache[key] = { base: baseId, full, width, height, signatures, snaps: [] };
    image = start.clone() as AbraImage;
  }
  const cache = renderCache[key]!;
  try {
    runSteps(image, steps, base, from, (index, state) => {
      if (!cache.snaps[index]) cache.snaps[index] = state.clone() as AbraImage;
    });
  } catch (error) {
    clearRenderCacheFor(full, variant);
    throw error;
  } finally {
    // The cache holds its own snapshots of it, and a native image the garbage collector cannot see the size of must not wait for it.
    release(image);
  }
  return cache.snaps[steps.length]!.clone() as AbraImage;
}

/**
 * Runs the steps on `image`, telling each how much smaller it is than `full`, the photo they are all edits of. Runs from step
 * `from`, which must be a point where no layers are waiting. Each step is committed before the next one so the cache can keep
 * a reusable composite for every editable control, including controls that paint a layer.
 */
function runSteps(
  image: AbraImage,
  steps: Step[],
  full: AbraImage | null,
  from = 0,
  capture?: (index: number, state: AbraImage) => void,
) {
  const scale = scaleBetween(image, full);
  // Steps with a layer wait here until a step that is not one comes (or the steps end), and then go on a canvas over the photo.
  let sheets: Sheet[] = [];
  const lay = () => {
    if (sheets.length === 0) return;
    const laid = withLayers(image, sheets);
    image.replaceWith(laid);
    // They are native images the garbage collector cannot see the size of.
    release(laid);
    sheets = [];
  };
  const timings: string[] = [];
  const timed = <T>(label: string, run: () => T): T => {
    const started = Date.now();
    const result = run();
    timings.push(`${label} ${Date.now() - started}`);
    return result;
  };
  if (from === 0) capture?.(0, image);
  for (let index = from; index < steps.length; index++) {
    const step = steps[index];
    if (step.sheet) {
      const sheet = step.sheet(image, scale);
      if (sheet.overEarlier) timed('layers', lay);
      sheets.push(sheet);
      timed('layers', lay);
      capture?.(index + 1, image);
      continue;
    }
    timed('layers', lay);
    timed(step.key, () => step.run(image, scale));
    capture?.(index + 1, image);
  }
  timed('layers', lay);
  devDebug('steps', ` ${image.width()}px:`, timings.join(', '));
}

/**
 * Whether every applied control, other than the slider if it is given, only makes the photo remember effects for the GPU. Then
 * the edit is made on the full-size photo at no cost, and that is what the editor draws; one control that needs the CPU makes
 * it a screen-size edit.
 */
export const isStackDeferred = (sliderKey?: string, pending?: ActionControl): boolean =>
  appliedSteps(
    pending?.group ? new Set([pending.group]) : undefined,
    sliderKey ? { key: sliderKey } : undefined,
    pending,
  ).every(step => step.deferred);

/** Whether any control other than the slider is applied, so a drag of it has to start from the stack rendered without it. */
export const hasOtherSteps = (sliderKey: string): boolean => appliedSteps(undefined, { key: sliderKey }).length > 0;

/**
 * The steps other than the slider's, split by where the slider sits in the stack (see `stageOf`): the ones that run before it
 * and the ones that run after it.
 */
function stepsAroundSlider(sliderKey: string) {
  const stages = EDIT_SECTIONS.flatMap(section =>
    leafControls(section.controls).flatMap(control =>
      control.kind === 'slider' && control.key === sliderKey ? [stageOf(section.key, !!control.layer)] : [],
    ),
  );
  const steps = appliedSteps(undefined, { key: sliderKey });
  return { before: steps.filter(step => step.stage <= stages[0]), after: steps.filter(step => step.stage > stages[0]) };
}

/**
 * The edit stack without one slider, rendered on the CPU: what a drag of that slider starts from, with the slider's effect
 * shown on top by the live preview. With `beforeOnly`, the steps that come after the slider in the stack (such as makeup over
 * the skin) are left out, and `renderStepsAfterSlider` puts them back over what the drag makes. The caller owns the result.
 */
export function renderStackWithoutSlider(sliderKey: string, beforeOnly = false, full = false): AbraImage | null {
  const base = editBaseImage.value;
  if (!base) return null;
  const steps = beforeOnly ? stepsAroundSlider(sliderKey).before : appliedSteps(undefined, { key: sliderKey });
  return renderCachedStack(base, full, steps, `slider:${sliderKey}:${beforeOnly ? 'before' : 'rest'}`);
}

/** Runs the steps that come after the slider in the stack on `image`, which a drag of the slider made from the steps before it. */
export function renderStepsAfterSlider(sliderKey: string, image: AbraImage) {
  const base = editBaseImage.value;
  if (base) runSteps(image, stepsAroundSlider(sliderKey).after, base);
}

/**
 * What a live preview of tapping this action starts from: the edit stack without the action's exclusive group (picking the
 * action replaces the others in it), rendered on the CPU. `null` when nothing else is applied, so the preview goes on the
 * untouched edit base. The caller owns the result.
 */
export function restForAction(control: ActionControl, full = false): (() => AbraImage) | null {
  const base = editBaseImage.value;
  if (!base) return null;
  const groups = control.group ? new Set([control.group]) : undefined;
  if (appliedSteps(groups).filter(step => control.repeatable || step.key !== control.key).length === 0) return null;
  const steps = appliedSteps(groups);
  return () => renderCachedStack(base, full, steps, `action:${control.key}`);
}

const imageIds = new WeakMap<object, number>();
let lastImageId = 0;
const imageId = (image: object) => {
  if (!imageIds.has(image)) imageIds.set(image, ++lastImageId);
  return imageIds.get(image)!;
};
let cachedThumbnailKey = '';
/** The edit base the preloaded images were made from. */
let preloadedBase: AbraImage | null = null;
/** The edit base the last screen-size replay was made from. */
let replayedBase: AbraImage | null = null;
let preloadTimer: ReturnType<typeof setTimeout> | undefined;

/**
 * Makes what the focused control (and the controls worked together with it, such as the tabs of Pop Art) preloads, from the screen-size photo, when it has a `preload` and has not made it for this photo.
 * Only the control in use is preloaded, never the rest of its section: a section can hold a control with a slow AI preload (Pop Art in
 * Special Effects) that a user working with another control never asked for. An edit that is already applied preloads when it is
 * replayed. A slow preload (the face models of Beauty) runs in the background, and the edit is drawn again when it is made.
 */
function preloadSection() {
  const base = editBaseImage.value;
  if (!base) return;
  const focused = focusedControlKey.value;
  const controls = focused
    ? controlsWith(findSection(activeSectionKey.value).controls, focused).filter(control => control.preload)
    : [];
  if (controls.length === 0) return;
  if (preloadedBase && preloadedBase !== base) clearPreloads();
  preloadedBase = base;
  const shrunk = shrunkOf('base', base);
  for (const control of controls) preloadFor(control, shrunk, shrunk);
}

let thumbnailBuild: ReturnType<typeof buildControlThumbnails> | undefined;
let thumbnailFrame: number | undefined;
let thumbnailTimer: ReturnType<typeof setTimeout> | undefined;
/** Starts the pictures of the pickers inside a mixed control, by the control's key, for when the control is opened. */
const lazyPickerBuilds = new Map<string, () => void>();
/** The picker builds that have started, so a change of the edit stops them like the rest. */
const openedPickerBuilds: ReturnType<typeof buildControlThumbnails>[] = [];

function cancelThumbnailBuild() {
  lazyPickerBuilds.clear();
  openedPickerBuilds.splice(0).forEach(each => each.cancel());
  if (thumbnailFrame !== undefined) cancelAnimationFrame(thumbnailFrame);
  if (thumbnailTimer !== undefined) clearTimeout(thumbnailTimer);
  thumbnailFrame = undefined;
  thumbnailTimer = undefined;
  thumbnailBuild?.cancel();
  thumbnailBuild = undefined;
}

function replaceThumbnails(thumbnails: Record<string, ThumbnailPreview>) {
  const previous = controlThumbnails.value;
  controlThumbnails.next(thumbnails);
  const retired = Object.entries(previous).filter(([key, thumbnail]) => thumbnails[key] !== thumbnail);
  if (retired.length === 0) return;
  // Let React render the new set before freeing previews held by the old render. Use a timer rather than requestAnimationFrame:
  // teardown can stop drawing frames, which would otherwise keep every retired thumbnail alive indefinitely.
  setTimeout(() => {
    for (const [, thumbnail] of retired) release(thumbnail);
  }, 0);
}

/** Clears the published sources before releasing them, including the replay cache on session teardown. */
export function clearControlThumbnails() {
  cancelThumbnailBuild();
  cachedThumbnailKey = '';
  replaceThumbnails({});
}

/**
 * Thumbnails for `section`'s controls. Controls in an exclusive group (e.g. colorizations) preview
 * against the image *without* that group applied, so picking one doesn't change the others' previews;
 * everything else previews against the current image. Reused as-is while their inputs are unchanged.
 */
function sectionThumbnails(section: EditSection, base: AbraImage) {
  if (!section.previewThumbnails) {
    cachedThumbnailKey = '';
    replaceThumbnails({});
    return;
  }
  const groups = new Set<string>();
  for (const control of section.controls) if (control.kind === 'action' && control.group) groups.add(control.group);
  const applied = appliedActions.value;
  const inGroup = applied.filter(key => groups.has(findAction(key)?.control.group ?? ''));
  const key = JSON.stringify([
    section.key,
    imageId(base),
    applied.filter(k => !inGroup.includes(k)),
    adjustments.value,
  ]);
  if (key === cachedThumbnailKey) return;
  cachedThumbnailKey = '';

  // The edit is replayed on the cached thumbnail-size copy of the photo, so nothing here shrinks a picture again.
  // It is made from the screen-size copy, not the full-size photo, so a large photo costs no more to shrink than a small one.
  const started = Date.now();
  const source = shrunkOf('thumbnail', shrunkOf('base', base), thumbnailPixels()).clone() as AbraImage;
  try {
    runSteps(source, appliedSteps(inGroup.length ? groups : undefined), base);
    devDebug('thumbnails', 'small-size replay ms:', Date.now() - started);
    const previous = Object.fromEntries(
      section.controls.flatMap(control => {
        const preview = controlThumbnails.value[control.key];
        return preview ? [[control.key, preview]] : [];
      }),
    );
    replaceThumbnails(previous);
    const build = buildControlThumbnails(
      source,
      section,
      previous,
      (controlKey, preview) => {
        batch(() => replaceThumbnails({ ...controlThumbnails.value, [controlKey]: preview }));
      },
      base,
    );
    // The pictures of a picker inside a mixed control show the effect with and without a part, so each starts from the edit without
    // the sliders the picker saves to, or the effect would be drawn twice.
    // They are only seen once the control is open, so they are made then (see `lazyPickerBuilds`) and not slowing the cards of the row.
    const builds = [build];
    for (const control of thumbnailControls(section.controls)) {
      if (control.kind !== 'mixed') continue;
      const pickers = leafControls(control.controls).filter(
        (inner): inner is PickerControl => inner.kind === 'picker' && !!inner.commitTo,
      );
      if (pickers.length === 0) continue;
      let started = false;
      const start = () => {
        if (started) return;
        started = true;
        for (const sliderKey of new Set(pickers.map(picker => picker.commitTo!.slider))) {
          const rest = shrunkOf('thumbnail', shrunkOf('base', base), thumbnailPixels()).clone() as AbraImage;
          try {
            runSteps(rest, appliedSteps(undefined, { key: sliderKey }), base);
          } catch (error) {
            release(rest);
            throw error;
          }
          const own = { ...section, controls: pickers.filter(picker => picker.commitTo!.slider === sliderKey) };
          const slider = leafControls(section.controls).find(
            (inner): inner is SliderControl => inner.kind === 'slider' && inner.key === sliderKey,
          );
          openedPickerBuilds.push(
            buildControlThumbnails(
              rest,
              own,
              {},
              (controlKey, preview) => {
                batch(() => replaceThumbnails({ ...controlThumbnails.value, [controlKey]: preview }));
              },
              base,
              slider && sliderValues(slider, adjustments.value[sliderKey]),
              build.done,
            ),
          );
        }
      };
      // What is focused when the control is open is one of its sliders, not the mixed control, so any of them starts the build.
      const keys = [
        control.key,
        ...leafControls(control.controls).flatMap(inner => (inner.kind === 'slider' ? [inner.key] : [])),
      ];
      if (keys.includes(focusedControlKey.value ?? '')) start();
      else for (const each of keys) lazyPickerBuilds.set(each, start);
    }
    const all = {
      done: Promise.all(builds.map(each => each.done)).then(results => results.every(Boolean)),
      cancel: () => builds.forEach(each => each.cancel()),
    };
    thumbnailBuild = all;
    void all.done.then(succeeded => {
      if (thumbnailBuild !== all) return;
      thumbnailBuild = undefined;
      cachedThumbnailKey = succeeded ? key : '';
    });
  } catch (error) {
    release(source);
    throw error;
  }
}

function replay() {
  // Re-check: on Fast Refresh this can fire after the session was disposed and its images destroyed.
  const base = editBaseImage.value;
  replayHidden.next(false);
  if (!base) {
    busy.next(false);
    return;
  }

  try {
    // A new photo makes what was preloaded from the old one again, when its control is next used. This comes before anything is
    // drawn, so the edits drawn now (and the thumbnails after them) share what is made for the new photo and do not lose it.
    if (preloadedBase && preloadedBase !== base) clearPreloads();
    preloadedBase = base;
    replayedBase = base;

    const started = Date.now();
    // When every control only makes the photo remember effects, the edit is made on the full-size photo, which costs nothing and
    // is what is drawn and saved. Otherwise it is made on a screen-size copy first, and on the full-size photo a moment later.
    const deferred = isStackDeferred();
    const recipe: ProjectRecipe = { adjustments: adjustments.value, appliedActions: appliedActions.value };
    const hasRecipe = Object.keys(recipe.adjustments).length > 0 || recipe.appliedActions.length > 0;
    if (
      currentImageRecipe === recipeSignature(recipe) &&
      currentImageBase === base &&
      (!hasRecipe || currentImage.value)
    ) {
      // A reopened project's full-size result is already current. Draw from the restored screen cache and leave its saved
      // pixels alone; loading a project must not run its controls again just to recreate the image already on disk.
      const next = renderCachedStack(base, false);
      setShown(next);
      fullReplayPending = false;
      devDebug('preview', 'restored saved render cache without running controls');
    } else {
      const next = renderCachedStack(base, deferred);
      // Keeps a held before/after preview correct even if an edit finishes replaying with a finger down.
      setShown(next);
      if (deferred) {
        fullReplayPending = false;
        release(currentImage.value);
        currentImage.next(next.clone() as AbraImage);
        currentImageRecipe = recipeSignature(recipe);
        currentImageBase = base;
      } else fullReplayPending = true;
      devDebug('preview', ` ${deferred ? 'full-size deferred' : 'screen-size'} replay ms:`, Date.now() - started);
    }

    // Let React mount the main preview before starting thumbnail work.
    thumbnailFrame = requestAnimationFrame(() => {
      thumbnailFrame = undefined;
      thumbnailTimer = setTimeout(() => {
        thumbnailTimer = undefined;
        try {
          preloadSection();
          sectionThumbnails(findSection(activeSectionKey.value), base);
        } catch (error) {
          warn('edit', 'preparing thumbnail edit stack threw:', error);
        }
      }, 0);
    });
  } catch (cause) {
    error('edit', 'preview replay failed:', cause);
  } finally {
    // A failed native or tool render must not leave the editor permanently busy; keep the last preview interactive.
    busy.next(false);
  }
}

/**
 * Replays the whole edit stack onto a fresh copy of the edit base whenever it, the active section
 * (whose thumbnails, if any, must reflect the latest image) or the preview size changes. Every
 * change replays from scratch, so moving a slider back to 0 truly removes its effect.
 *
 * Full-resolution replay is deferred so immediate feedback can paint. Thumbnail work starts in a
 * separate timer after the main preview's update; async native results publish progressively.
 */
export function startReplay(): Subscription {
  // Keep the current box for the first replay, then only publish it after its height tween settles.
  // Other replay inputs (especially committed edits) still trigger immediately.
  const settledPreviewBox$ = previewBox$.pipe(debounceTime(REPLAY_BOX_SETTLE_MS), startWith(previewBox.value));
  const subscription = combineLatest([
    ready$,
    adjustments$,
    appliedActions$,
    activeSectionKey$,
    settledPreviewBox$,
    analysisVersion$,
    historyVersion$,
  ])
    .pipe(
      distinctUntilChanged(
        (a, b) =>
          a[0] === b[0] &&
          a[1] === b[1] &&
          a[2] === b[2] &&
          a[3] === b[3] &&
          a[4]?.width === b[4]?.width &&
          a[4]?.height === b[4]?.height &&
          a[5] === b[5] &&
          a[6] === b[6],
      ),
      filter(([isReady, , , , box]) => isReady && !!box && !!editBaseImage.value),
      tap(() => {
        // The screen cache remains reusable through preview-box animations and mask discovery. A control change
        // updates the affected suffix by signature; section/layout changes only refresh their thumbnails.
        cancelThumbnailBuild();
        devDebug('replay', 'change received', activeSectionKey.value);
        busy.next(!replayHidden.value);
      }),
      switchMap(
        () =>
          new Observable<void>(subscriber => {
            // React commits the tap's own UI (selection outline, busy indicator) within a frame, but
            // the native side still needs a moment to draw it before the replay blocks the JS thread
            // on synchronous native work. Back-to-back animation frames fire too close together to
            // leave that gap, so wait a frame and then a short real delay.
            let timer: ReturnType<typeof setTimeout> | undefined;
            const frame = requestAnimationFrame(() => {
              timer = setTimeout(() => {
                batch(replay);
                subscriber.complete();
              }, REPLAY_DELAY_MS);
            });
            return () => {
              cancelAnimationFrame(frame);
              clearTimeout(timer);
            };
          }),
      ),
    )
    .subscribe();
  // Clicking a control preloads it, after a frame so the control's own UI is drawn first.
  subscription.add(
    focusedControlKey$.subscribe(key => {
      const start = key ? lazyPickerBuilds.get(key) : undefined;
      if (key) lazyPickerBuilds.delete(key);
      start?.();
      clearTimeout(preloadTimer);
      preloadTimer = setTimeout(preloadSection, 0);
    }),
  );
  subscription.add(() => clearTimeout(preloadTimer));
  subscription.add(cancelThumbnailBuild);
  return subscription;
}
