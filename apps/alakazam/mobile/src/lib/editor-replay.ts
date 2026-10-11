import { batch } from '@/src/lib/batch';
import {
  applyAction,
  applySlider,
  EDIT_SECTIONS,
  findSection,
  isSliderApplied,
  scaleBetween,
  sliderDefault,
  type AdjustmentValue,
  type ActionControl,
  type EditSection,
} from '@/src/lib/edit-sections';
import { releasePictures, setShown, shrunkOf } from '@/src/lib/editor-pictures';
import { clearPreloads, preloadFor } from '@/src/lib/editor-preloads';
import { buildControlThumbnails } from '@/src/lib/editor-thumbnails';
import { type ProjectRecipe } from '@/src/lib/project-store';
import { adjustmentKey, isSkinAdjustment, skinAdjustmentTargets } from '@/src/lib/skin-adjustments';
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
import { AbraImage, type AbraImageLike, type ThumbnailPreview } from '@alakazam/mobile';
import { combineLatest, Observable, type Subscription } from 'rxjs';
import { filter, switchMap, tap } from 'rxjs/operators';

// #region: The full-size picture, which is not shown

/** Gap before the full-size edit starts, so a run of quick edits (a slider moved again) only makes the last one. */
const FULL_REPLAY_DELAY_MS = 150;
let fullReplayTimer: ReturnType<typeof setTimeout> | undefined;

/** Makes the edit on the full-size photo. It is not shown: it is `currentImage`, for what needs every pixel. */
function runFullReplay() {
  fullReplayTimer = undefined;
  const base = editBaseImage.value;
  if (!base) return;
  const started = Date.now();
  const next = renderEditStack(base, undefined, true);
  currentImage.value?.uniffiDestroy();
  currentImage.next(next);
  if (__DEV__) console.debug('[edit] full-size replay ms (not shown):', Date.now() - started);
}

function scheduleFullReplay() {
  if (fullReplayTimer !== undefined) clearTimeout(fullReplayTimer);
  fullReplayTimer = setTimeout(runFullReplay, FULL_REPLAY_DELAY_MS);
}

/**
 * Makes the full-size edit now if one is waiting. Whatever changes the full-size photo in place, such as the blemish tool,
 * calls this first, so it works on the photo as it is edited now and not on the one before the last release.
 */
export function flushFullReplay() {
  if (fullReplayTimer === undefined) return;
  clearTimeout(fullReplayTimer);
  runFullReplay();
}

function cancelFullReplay() {
  if (fullReplayTimer !== undefined) clearTimeout(fullReplayTimer);
  fullReplayTimer = undefined;
}

/** Lets go of every screen-size picture and stops a waiting full-size edit, when the session ends. */
export function releaseShownImages() {
  cancelFullReplay();
  clearPreloads();
  preloadedBase = null;
  releasePictures();
}

// #endregion

/** Gap between an edit and its replay, so the tap's immediate UI feedback can be drawn first. */
const REPLAY_DELAY_MS = 60;

const findAction = (key: string): ActionControl | undefined => {
  for (const section of EDIT_SECTIONS) {
    for (const control of section.controls) if (control.kind === 'action' && control.key === key) return control;
  }
  return undefined;
};

/** One applied control: how to run it on an image. */
type Step = {
  key: string;
  /** The slider's value; actions have none. */
  value?: AdjustmentValue;
  run: (image: AbraImageLike, scale: number) => void;
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
    const control = findAction(key);
    if (!control || (!control.apply && !control.live && !control.operation && !control.tool)) continue;
    // The action about to be applied replaces the others in its group, so it is never left out with them.
    if (control !== pending && control.group && excludeGroups?.has(control.group)) continue;
    steps.push({ key, run: (image, scale) => applyAction(control, image, scale) });
  }
  for (const section of EDIT_SECTIONS) {
    for (const control of section.controls) {
      if (control.kind !== 'slider') continue;
      if (isSkinAdjustment(control.key)) {
        const effect = control.live;
        if (!effect) throw new Error(`Skin control has no effect definition: ${control.key}`);
        const activeKey = adjustmentKey(control.key, personSelection.value.selectedId);
        for (const target of skinAdjustmentTargets(recipe.adjustments, control.key)) {
          if (slider?.key === control.key && target.key === activeKey) continue;
          if (!isSliderApplied(control, target.value)) continue;
          steps.push({
            key: target.key,
            value: target.value,
            run: image => {
              image.applyEffectToPerson(effect(target.value), target.personId ?? undefined);
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
        run: (image, scale) => applySlider(control, image, value, scale),
      });
    }
  }
  return steps;
}

/** Renders a saved recipe onto a fresh copy of `base`, without touching the live edit state. The caller owns the result. */
export function renderRecipe(base: AbraImage, recipe: ProjectRecipe): AbraImage {
  const next = base.copy() as AbraImage;
  runSteps(next, appliedSteps(undefined, undefined, undefined, recipe), base);
  return next;
}

/**
 * Renders the whole edit stack onto a fresh copy of the edit base, shrunk to the screen unless `full` is set. The caller owns
 * the result.
 */
function renderEditStack(base: AbraImage, excludeGroups?: ReadonlySet<string>, full = false) {
  const next = (full ? base : shrunkOf('base', base)).copy() as AbraImage;
  runSteps(next, appliedSteps(excludeGroups), base);
  return next;
}

/** Runs the steps on `image`, telling each how much smaller it is than `full`, the photo they are all edits of. */
function runSteps(image: AbraImage, steps: Step[], full: AbraImage | null) {
  const scale = scaleBetween(image, full);
  for (const step of steps) step.run(image, scale);
}

/** Whether any control other than the slider is applied, so a drag of it has to start from the stack rendered without it. */
export const hasOtherSteps = (sliderKey: string): boolean => appliedSteps(undefined, { key: sliderKey }).length > 0;

/**
 * The edit stack without one slider, rendered on the CPU: what a drag of that slider starts from, with the slider's effect
 * shown on top by the live preview. The caller owns the result.
 */
export function renderStackWithoutSlider(sliderKey: string): AbraImage | null {
  const base = editBaseImage.value;
  if (!base) return null;
  const next = shrunkOf('base', base).copy() as AbraImage;
  runSteps(next, appliedSteps(undefined, { key: sliderKey }), base);
  return next;
}

/**
 * What a live preview of tapping this action starts from: the edit stack without the action's exclusive group (picking the
 * action replaces the others in it), rendered on the CPU. `null` when nothing else is applied, so the preview goes on the
 * untouched edit base. The caller owns the result.
 */
export function restForAction(control: ActionControl): (() => AbraImage) | null {
  const base = editBaseImage.value;
  if (!base) return null;
  const groups = control.group ? new Set([control.group]) : undefined;
  if (appliedSteps(groups).filter(step => step.key !== control.key).length === 0) return null;
  return () => renderEditStack(base, groups);
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
let preloadTimer: ReturnType<typeof setTimeout> | undefined;

/**
 * Makes what the focused control preloads, from the screen-size photo, if it has a `preload` and has not made it for this photo.
 * A control is preloaded when it is clicked and not before, so nothing is made for the controls that are never used.
 */
function preloadFocused() {
  const base = editBaseImage.value;
  const control = findSection(activeSectionKey.value).controls.find(item => item.key === focusedControlKey.value);
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
function sectionThumbnails(section: EditSection, base: AbraImage, current: AbraImage) {
  if (!section.previewThumbnails) {
    cachedThumbnailKey = '';
    replaceThumbnails({});
    return;
  }
  const groups = new Set<string>();
  for (const control of section.controls) if (control.kind === 'action' && control.group) groups.add(control.group);
  const applied = appliedActions.value;
  const inGroup = applied.filter(key => groups.has(findAction(key)?.group ?? ''));
  const key = JSON.stringify([
    section.key,
    imageId(base),
    applied.filter(k => !inGroup.includes(k)),
    adjustments.value,
  ]);
  if (key === cachedThumbnailKey) return;
  cachedThumbnailKey = '';

  const source = inGroup.length ? renderEditStack(base, groups) : current;
  try {
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
  } finally {
    if (source !== current) source.uniffiDestroy();
  }
}

function replay() {
  // Re-check: on Fast Refresh this can fire after the session was disposed and its images destroyed.
  const base = editBaseImage.value;
  replayHidden.next(false);
  if (!base) return busy.next(false);

  const started = Date.now();
  const next = renderEditStack(base);
  // Keeps a held before/after preview correct even if an edit finishes replaying with a finger down.
  setShown(next);
  if (__DEV__) console.debug('[preview] screen-size replay ms:', Date.now() - started);
  scheduleFullReplay();

  busy.next(false);
  // Let React mount the main preview before starting thumbnail work.
  thumbnailFrame = requestAnimationFrame(() => {
    thumbnailFrame = undefined;
    thumbnailTimer = setTimeout(() => {
      thumbnailTimer = undefined;
      try {
        // A new photo makes what was preloaded from the old one again, when its control is next used.
        if (preloadedBase && preloadedBase !== base) clearPreloads();
        preloadedBase = base;
        preloadFocused();
        sectionThumbnails(findSection(activeSectionKey.value), base, next);
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
  subscription.add(cancelFullReplay);
  return subscription;
}
