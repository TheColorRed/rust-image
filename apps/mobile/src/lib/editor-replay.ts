import { AbraImage } from '@alakazam/mobile';
import { combineLatest, Observable, type Subscription } from 'rxjs';
import { filter, switchMap, tap } from 'rxjs/operators';
import { batch } from '@/src/lib/batch';
import { EDIT_SECTIONS, findSection, type EditSection } from '@/src/lib/edit-sections';
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
import { busy, currentImage, editBaseImage, ready$ } from '@/src/state/session';

/** Gap between an edit and its replay, so the tap's immediate UI feedback can be drawn first. */
const REPLAY_DELAY_MS = 60;

const actionGroup = (key: string) => {
  for (const section of EDIT_SECTIONS) {
    for (const control of section.controls) if (control.kind === 'action' && control.key === key) return control.group;
  }
  return undefined;
};

/**
 * Renders the whole edit stack (one-shot actions, then sliders) onto a fresh copy of the edit base.
 * `excludeGroups` leaves out applied actions of those mutually exclusive groups.
 */
function renderEditStack(base: AbraImage, excludeGroups?: ReadonlySet<string>, skipSliderKey?: string) {
  const next = base.copy() as AbraImage;
  for (const key of appliedActions.value) {
    const group = actionGroup(key);
    if (group && excludeGroups?.has(group)) continue;
    for (const section of EDIT_SECTIONS) {
      const action = section.controls.find(control => control.kind === 'action' && control.key === key);
      if (action?.kind === 'action') action.apply(next);
    }
  }
  for (const section of EDIT_SECTIONS) {
    for (const control of section.controls) {
      if (control.kind !== 'slider' || control.key === skipSliderKey) continue;
      const value = adjustments.value[control.key] ?? control.defaultValue;
      if (value !== control.defaultValue) control.apply(next, value);
    }
  }
  return next;
}

/** The edit stack without one slider: the starting point for previewing that slider live. The caller owns the result. */
export function renderEditStackWithoutSlider(sliderKey: string): AbraImage | null {
  const base = editBaseImage.value;
  return base ? renderEditStack(base, undefined, sliderKey) : null;
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
  const inGroup = applied.filter(key => groups.has(actionGroup(key) ?? ''));
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
      tap(() => busy.next(true)),
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
