import { batch } from '@/src/lib/batch';
import {
  EDIT_SECTIONS,
  applyAction,
  applySlider,
  findSection,
  isSliderApplied,
  type ActionControl,
  type EditSection,
} from '@/src/lib/edit-sections';
import { comparisonImage, recordDraftStep } from '@/src/lib/editor-history';
import { adjustmentKey, isSkinAdjustment, skinAdjustmentTargets } from '@/src/lib/skin-adjustments';
import { personSelection } from '@/src/state/person-selection';
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
import { AbraImage, type AbraImageLike, type ThumbnailPreview } from '@alakazam/mobile';
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

/** One applied control: how to run it on an image. */
type Step = {
  key: string;
  /** The slider's value; actions have none. */
  value?: number;
  run: (image: AbraImageLike) => void;
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
  const keys =
    pending && !appliedActions.value.includes(pending.key)
      ? [...appliedActions.value, pending.key]
      : appliedActions.value;
  for (const key of keys) {
    const control = findAction(key);
    if (!control || (!control.apply && !control.live && !control.operation)) continue;
    // The action about to be applied replaces the others in its group, so it is never left out with them.
    if (control !== pending && control.group && excludeGroups?.has(control.group)) continue;
    steps.push({ key, run: image => applyAction(control, image) });
  }
  for (const section of EDIT_SECTIONS) {
    for (const control of section.controls) {
      if (control.kind !== 'slider') continue;
      if (isSkinAdjustment(control.key)) {
        const effect = control.live;
        if (!effect) throw new Error(`Skin control has no effect definition: ${control.key}`);
        const activeKey = adjustmentKey(control.key, personSelection.value.selectedId);
        for (const target of skinAdjustmentTargets(adjustments.value, control.key)) {
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
      const committed = dragged ? slider.value : adjustments.value[control.key];
      if (!dragged && !isSliderApplied(control, committed)) continue;
      const value = committed ?? control.defaultValue;
      steps.push({
        key: control.key,
        value,
        run: image => applySlider(control, image, value),
      });
    }
  }
  return steps;
}

/** Renders the whole edit stack onto a fresh copy of the edit base. */
function renderEditStack(base: AbraImage, excludeGroups?: ReadonlySet<string>) {
  const next = base.copy() as AbraImage;
  runSteps(next, appliedSteps(excludeGroups));
  return next;
}

function runSteps(image: AbraImage, steps: Step[]) {
  for (const step of steps) step.run(image);
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
  const next = base.copy() as AbraImage;
  runSteps(next, appliedSteps(undefined, { key: sliderKey }));
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
  for (const [key, thumbnail] of Object.entries(previous)) {
    if (thumbnails[key] !== thumbnail) thumbnail.uniffiDestroy();
  }
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
    const build = buildControlThumbnails(source, section, previous, (controlKey, preview) => {
      batch(() => replaceThumbnails({ ...controlThumbnails.value, [controlKey]: preview }));
    });
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
  currentImage.value?.uniffiDestroy();
  currentImage.next(next);
  recordDraftStep(next);

  // Keep a held before/after preview correct even if an edit finishes replaying with a finger down.
  previewSourceImage.next(showingCheckpoint.value ? (comparisonImage() ?? next) : next);
  if (__DEV__) console.debug('[preview] full-resolution replay ms:', Date.now() - started);

  busy.next(false);
  // Let React mount the main preview before starting thumbnail work.
  thumbnailFrame = requestAnimationFrame(() => {
    thumbnailFrame = undefined;
    thumbnailTimer = setTimeout(() => {
      thumbnailTimer = undefined;
      try {
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
  subscription.add(cancelThumbnailBuild);
  return subscription;
}
