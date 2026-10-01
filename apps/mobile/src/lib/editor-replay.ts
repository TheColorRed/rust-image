import { batch } from '@/src/lib/batch';
import {
  EDIT_SECTIONS,
  applyAction,
  findSection,
  isSliderApplied,
  type ActionControl,
  type EditSection,
} from '@/src/lib/edit-sections';
import { comparisonImage, recordDraftStep } from '@/src/lib/editor-history';
import { buildControlThumbnails } from '@/src/lib/editor-thumbnails';
import {
  activeSectionKey,
  activeSectionKey$,
  adjustments,
  adjustments$,
  appliedActions,
  appliedActions$,
} from '@/src/state/edits';
import { previewBox$ } from '@/src/state/gestures';
import { controlThumbnails, previewSourceImage, showingCheckpoint } from '@/src/state/preview';
import { busy, currentImage, editBaseImage, ready$, replayHidden } from '@/src/state/session';
import { AbraImage, type AbraImageLike, type EffectSpec } from '@alakazam/mobile';
import { combineLatest, Observable, type Subscription } from 'rxjs';
import { filter, switchMap, tap } from 'rxjs/operators';

/** Gap between an edit and its replay, so the tap's immediate UI feedback can be drawn first. */
const REPLAY_DELAY_MS = 60;

const findAction = (key: string): ActionControl | undefined => {
  for (const section of EDIT_SECTIONS) {
    for (const control of section.controls) if (control.kind === 'action' && control.key === key) return control;
  }
  return undefined;
};

/** One applied control: how to run it on an image, and, when it has a live form, its effect. */
type Step = {
  key: string;
  /** The slider's value; actions have none. */
  value?: number;
  run: (image: AbraImageLike) => void;
  live?: () => EffectSpec;
};

/**
 * The applied controls in the order they run: one-shot actions, then sliders in section order.
 * `excludeGroups` leaves out applied actions of those mutually exclusive groups. `slider` is the slider being dragged:
 * without a `value` it is left out, and with one it is included at that value whether or not it is applied.
 * `pending` is an action about to be applied: it goes after the applied actions, where toggling it will put it.
 */
function appliedSteps(
  excludeGroups?: ReadonlySet<string>,
  slider?: { key: string; value?: number },
  pending?: ActionControl,
): Step[] {
  const steps: Step[] = [];
  const keys = pending && !appliedActions.value.includes(pending.key) ? [...appliedActions.value, pending.key] : appliedActions.value;
  for (const key of keys) {
    const control = findAction(key);
    if (!control || (!control.apply && !control.live)) continue;
    if (control.group && excludeGroups?.has(control.group)) continue;
    steps.push({ key, run: image => applyAction(control, image), live: control.live });
  }
  for (const section of EDIT_SECTIONS) {
    for (const control of section.controls) {
      if (control.kind !== 'slider') continue;
      const dragged = slider?.key === control.key;
      if (dragged && slider.value === undefined) continue;
      const committed = dragged ? slider.value : adjustments.value[control.key];
      if (!dragged && !isSliderApplied(control, committed)) continue;
      const value = committed ?? control.defaultValue;
      const live = control.live;
      steps.push({
        key: control.key,
        value,
        run: image => control.apply(image, value),
        live: live && (() => live(value)),
      });
    }
  }
  return steps;
}

/** Renders the whole edit stack onto a fresh copy of the edit base. */
function renderEditStack(base: AbraImage, excludeGroups?: ReadonlySet<string>) {
  const next = base.copy() as AbraImage;
  for (const step of appliedSteps(excludeGroups)) step.run(next);
  return next;
}

/**
 * Whether a drag of this slider can be previewed as one GPU chain over the original image: every applied control other
 * than the slider has a live form. Once more effects have shaders this is true more often; until then a stack with a
 * CPU-only effect falls back to rendering the rest on the CPU.
 */
export function canChainLive(sliderKey: string): boolean {
  return appliedSteps(undefined, { key: sliderKey }).every(step => step.live);
}

/**
 * What a live preview of this slider starts from. When the stack can run as a chain it is the untouched edit base;
 * otherwise it is the edit stack without the slider, and the slider's effect is applied on top. The caller owns the result.
 */
export function renderLivePreviewBase(sliderKey: string): AbraImage | null {
  const base = editBaseImage.value;
  if (!base) return null;
  return canChainLive(sliderKey) ? (base.copy() as AbraImage) : renderEditStackWithoutSlider(base, sliderKey);
}

function renderEditStackWithoutSlider(base: AbraImage, sliderKey: string) {
  const next = base.copy() as AbraImage;
  for (const step of appliedSteps(undefined, { key: sliderKey })) step.run(next);
  return next;
}

/**
 * The applied controls as effects, in the order they run, with the slider being dragged in its place at `value`. Only
 * valid when {@link canChainLive} is true.
 */
export function liveChain(sliderKey: string, value: number): { key: string; effect: EffectSpec }[] {
  return appliedSteps(undefined, { key: sliderKey, value }).map(step => ({ key: step.key, effect: step.live!() }));
}

/**
 * What an action's live preview is made of. Picking the action replaces the others in its exclusive group.
 *
 * When every control that will be applied has a live form, `image` is the untouched edit base and `chain` is the whole
 * stack as effects, with the action where the replay will put it. Otherwise `chain` is null and `image` is the stack
 * without the action's group, with the action's own effect to be applied on top (after the sliders, not before them as
 * in the replay, which is the best a CPU-rendered base allows). The caller owns `image`.
 */
export function renderPreviewForAction(
  control: ActionControl,
): { image: AbraImage; chain: { key: string; effect: EffectSpec }[] | null } | null {
  const base = editBaseImage.value;
  if (!base) return null;
  const groups = control.group ? new Set([control.group]) : undefined;
  const steps = appliedSteps(groups, undefined, control);
  if (steps.every(step => step.live)) {
    return { image: base.copy() as AbraImage, chain: steps.map(step => ({ key: step.key, effect: step.live!() })) };
  }
  return { image: renderEditStack(base, groups), chain: null };
}

const imageIds = new WeakMap<object, number>();
let lastImageId = 0;
const imageId = (image: object) => {
  if (!imageIds.has(image)) imageIds.set(image, ++lastImageId);
  return imageIds.get(image)!;
};
let cachedThumbnailKey = '';
let cachedThumbnails: Record<string, string> = {};

/**
 * Thumbnails for `section`'s controls. Controls in an exclusive group (e.g. colorizations) preview
 * against the image *without* that group applied, so picking one doesn't change the others' previews;
 * everything else previews against the current image. Reused as-is while their inputs are unchanged.
 */
function sectionThumbnails(section: EditSection, base: AbraImage, current: AbraImage) {
  if (!section.previewThumbnails) return {};
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
  if (key === cachedThumbnailKey) return cachedThumbnails;

  const source = inGroup.length ? renderEditStack(base, groups) : current;
  cachedThumbnails = buildControlThumbnails(source, section);
  cachedThumbnailKey = key;
  if (source !== current) source.uniffiDestroy();
  return cachedThumbnails;
}

function replay() {
  // Re-check: on Fast Refresh this can fire after the session was disposed and its images destroyed.
  const base = editBaseImage.value;
  replayHidden.next(false);
  if (!base) return busy.next(false);

  const next = renderEditStack(base);
  currentImage.value?.uniffiDestroy();
  currentImage.next(next);
  recordDraftStep(next);

  // Keep a held before/after preview correct even if an edit finishes replaying with a finger down.
  previewSourceImage.next(showingCheckpoint.value ? (comparisonImage() ?? next) : next);

  controlThumbnails.next(sectionThumbnails(findSection(activeSectionKey.value), base, next));
  busy.next(false);
}

/**
 * Replays the whole edit stack onto a fresh copy of the edit base whenever it, the active section
 * (whose thumbnails, if any, must reflect the latest image) or the preview size changes. Every
 * change replays from scratch, so moving a slider back to 0 truly removes its effect.
 *
 * Deferred a frame so the busy indicator can paint before the synchronous native work starts, and
 * thumbnails are built in that same callback rather than a second one: two independent rAF-deferred
 * state updates landing in one native frame batch crashed React with "Should not already be working".
 */
export function startReplay(): Subscription {
  return combineLatest([ready$, adjustments$, appliedActions$, activeSectionKey$, previewBox$])
    .pipe(
      filter(([isReady, , , , box]) => isReady && !!box && !!editBaseImage.value),
      tap(() => busy.next(!replayHidden.value)),
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
}
