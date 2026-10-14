import { personIn } from '@/src/lib/analysis';
import { batch } from '@/src/lib/batch';
import {
  applyAction,
  applySlider,
  EDIT_SECTIONS,
  findSection,
  layerSheet,
  leafControls,
  isSliderApplied,
  scaleBetween,
  sliderContext,
  sliderDefault,
  sliderValues,
  type AdjustmentValue,
  type ActionControl,
  type EditSection,
} from '@/src/lib/edit-sections';
import { releasePictures, setShown, shrunkOf } from '@/src/lib/editor-pictures';
import { withLayers, type Sheet } from '@/src/lib/layers';
import { clearPreloads, preloadFor, releasePreloads } from '@/src/lib/editor-preloads';
import { buildControlThumbnails, thumbnailPixels } from '@/src/lib/editor-thumbnails';
import { type ProjectRecipe } from '@/src/lib/project-store';
import { adjustmentKey, isPersonAdjustment, personAdjustmentTargets } from '@/src/lib/skin-adjustments';
import {
  activeSectionKey,
  activeSectionKey$,
  adjustments,
  adjustments$,
  appliedActions,
  appliedActions$,
  focusedControlKey,
  focusedControlKey$,
} from '@/src/state/edits';
import { previewBox$ } from '@/src/state/gestures';
import { personSelection } from '@/src/state/person-selection';
import { controlThumbnails } from '@/src/state/preview';
import { busy, currentImage, editBaseImage, ready$, replayHidden } from '@/src/state/session';
import { type Image as AbraPixels } from '@abra/core/image';
import { AbraImage, type AbraImageLike, type ThumbnailPreview } from '@alakazam/mobile';
import { combineLatest, Observable, type Subscription } from 'rxjs';
import { filter, switchMap, tap } from 'rxjs/operators';

// #region: The full-size picture, which is not shown

/** Whether the edit has changed since the full-size picture (`currentImage`) was made, so it is out of date. */
let fullReplayPending = false;

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
  const next = renderEditStack(base, undefined, true);
  currentImage.value?.uniffiDestroy();
  currentImage.next(next);
  if (__DEV__) console.debug('[edit] full-size replay ms (not shown):', Date.now() - started);
}

/**
 * Makes the full-size edit now if the one there is out of date. Whatever needs the full-size photo, such as saving or the
 * blemish tool, calls this first, so it works on the photo as it is edited now and not on the one before the last change.
 */
export function flushFullReplay() {
  if (fullReplayPending) runFullReplay();
}

/** Lets go of every screen-size picture and forgets a full-size edit that was waiting, when the session ends. */
export function releaseShownImages() {
  fullReplayPending = false;
  releasePreloads();
  preloadedBase = null;
  replayedBase = null;
  releasePictures();
}

// #endregion

/** Gap between an edit and its replay, so the tap's immediate UI feedback can be drawn first. */
const REPLAY_DELAY_MS = 60;

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
    pending && !recipe.appliedActions.includes(pending.key)
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
      run: (image, scale) => applyAction(control, image, scale),
    });
  }
  for (const section of EDIT_SECTIONS) {
    for (const control of leafControls(section.controls)) {
      if (control.kind !== 'slider') continue;
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
      if (!dragged && !isSliderApplied(control, committed)) continue;
      const value = committed ?? sliderDefault(control);
      steps.push({
        key: control.key,
        value,
        stage: stageOf(section.key, !!control.layer),
        deferred: !!control.deferred && !control.layer,
        sheet: control.layer
          ? (image, scale) => layerSheet(control, sliderValues(control, value), sliderContext(control, image, scale))
          : undefined,
        run: (image, scale) => applySlider(control, image, value, scale),
      });
    }
  }
  // Array.sort keeps the order of steps with the same stage.
  return steps.sort((first, second) => first.stage - second.stage);
}

/** Renders a saved recipe onto a fresh copy of `base`, without touching the live edit state. The caller owns the result. */
export function renderRecipe(base: AbraImage, recipe: ProjectRecipe): AbraImage {
  const next = base.clone() as AbraImage;
  runSteps(next, appliedSteps(undefined, undefined, undefined, recipe), base);
  return next;
}

/**
 * Renders the whole edit stack onto a fresh copy of the edit base, shrunk to the screen unless `full` is set. The caller owns
 * the result.
 */
function renderEditStack(base: AbraImage, excludeGroups?: ReadonlySet<string>, full = false) {
  const next = (full ? base : shrunkOf('base', base)).clone() as AbraImage;
  runSteps(next, appliedSteps(excludeGroups), base);
  return next;
}

/** Runs the steps on `image`, telling each how much smaller it is than `full`, the photo they are all edits of. */
function runSteps(image: AbraImage, steps: Step[], full: AbraImage | null) {
  const scale = scaleBetween(image, full);
  // Steps with a layer wait here until a step that is not one comes (or the steps end), and then go on a canvas over the photo.
  let sheets: Sheet[] = [];
  const lay = () => {
    if (sheets.length === 0) return;
    const laid = withLayers(image, sheets);
    image.replaceWith(laid);
    // They are native images the garbage collector cannot see the size of.
    (laid as AbraPixels).uniffiDestroy();
    sheets = [];
  };
  const timings: string[] = [];
  const timed = <T>(label: string, run: () => T): T => {
    const started = Date.now();
    const result = run();
    timings.push(`${label} ${Date.now() - started}`);
    return result;
  };
  for (const step of steps) {
    if (step.sheet) {
      const sheet = step.sheet(image, scale);
      if (sheet.overEarlier) timed('layers', lay);
      sheets.push(sheet);
      continue;
    }
    timed('layers', lay);
    timed(step.key, () => step.run(image, scale));
  }
  timed('layers', lay);
  if (__DEV__) console.debug(`[steps] ${image.width()}px:`, timings.join(', '));
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
  const next = (full ? base : shrunkOf('base', base)).clone() as AbraImage;
  runSteps(next, beforeOnly ? stepsAroundSlider(sliderKey).before : appliedSteps(undefined, { key: sliderKey }), base);
  return next;
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
  if (appliedSteps(groups).filter(step => step.key !== control.key).length === 0) return null;
  return () => renderEditStack(base, groups, full);
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
 * Makes what the focused control preloads, from the screen-size photo, if it has a `preload` and has not made it for this photo.
 * A control is preloaded when it is clicked and not before, so nothing is made for the controls that are never used.
 */
function preloadFocused() {
  const base = editBaseImage.value;
  const control = leafControls(findSection(activeSectionKey.value).controls).find(item => item.key === focusedControlKey.value);
  if (!base || control?.kind !== 'slider' || !control.preload) return;
  if (preloadedBase && preloadedBase !== base) clearPreloads();
  preloadedBase = base;
  const shrunk = shrunkOf('base', base);
  preloadFor(control, shrunk.width(), shrunk);
}

let thumbnailBuild: ReturnType<typeof buildControlThumbnails> | undefined;
let thumbnailFrame: number | undefined;
let thumbnailTimer: ReturnType<typeof setTimeout> | undefined;

function cancelThumbnailBuild() {
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
  // A view that draws one of these is still mounted until React has rendered the new set, and it reads the thumbnail when it
  // renders and when a refresh runs its effects again, so they are freed after that, not now.
  requestAnimationFrame(() => {
    setTimeout(() => {
      for (const [, thumbnail] of retired) thumbnail.uniffiDestroy();
    }, 0);
  });
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
    if (__DEV__) console.debug('[thumbnails] small-size replay ms:', Date.now() - started);
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
    thumbnailBuild = build;
    void build.done.then(succeeded => {
      if (thumbnailBuild !== build) return;
      thumbnailBuild = undefined;
      cachedThumbnailKey = succeeded ? key : '';
    });
  } catch (error) {
    source.uniffiDestroy();
    throw error;
  }
}

function replay() {
  // Re-check: on Fast Refresh this can fire after the session was disposed and its images destroyed.
  const base = editBaseImage.value;
  replayHidden.next(false);
  if (!base) return busy.next(false);

  // A new photo makes what was preloaded from the old one again, when its control is next used. This comes before anything is
  // drawn, so the edits drawn now (and the thumbnails after them) share what is made for the new photo and do not lose it.
  if (preloadedBase && preloadedBase !== base) clearPreloads();
  preloadedBase = base;
  replayedBase = base;

  const started = Date.now();
  // When every control only makes the photo remember effects, the edit is made on the full-size photo, which costs nothing and
  // is what is drawn and saved. Otherwise it is made on a screen-size copy first, and on the full-size photo a moment later.
  const deferred = isStackDeferred();
  const next = renderEditStack(base, undefined, deferred);
  // Keeps a held before/after preview correct even if an edit finishes replaying with a finger down.
  setShown(next);
  if (deferred) {
    fullReplayPending = false;
    currentImage.value?.uniffiDestroy();
    currentImage.next(next.clone() as AbraImage);
  } else fullReplayPending = true;
  if (__DEV__) console.debug(`[preview] ${deferred ? 'full-size deferred' : 'screen-size'} replay ms:`, Date.now() - started);

  busy.next(false);
  // Let React mount the main preview before starting thumbnail work.
  thumbnailFrame = requestAnimationFrame(() => {
    thumbnailFrame = undefined;
    thumbnailTimer = setTimeout(() => {
      thumbnailTimer = undefined;
      try {
        preloadFocused();
        sectionThumbnails(findSection(activeSectionKey.value), base);
      } catch (error) {
        console.warn('[edit] preparing thumbnail edit stack threw:', error);
      }
    }, 0);
  });
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
  const subscription = combineLatest([ready$, adjustments$, appliedActions$, activeSectionKey$, previewBox$])
    .pipe(
      filter(([isReady, , , , box]) => isReady && !!box && !!editBaseImage.value),
      tap(() => {
        cancelThumbnailBuild();
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
    focusedControlKey$.subscribe(() => {
      clearTimeout(preloadTimer);
      preloadTimer = setTimeout(preloadFocused, 0);
    }),
  );
  subscription.add(() => clearTimeout(preloadTimer));
  subscription.add(cancelThumbnailBuild);
  return subscription;
}
