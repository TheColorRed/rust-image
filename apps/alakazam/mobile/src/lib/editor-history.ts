import { AbraImage, personDetectionDownloaded } from '@alakazam/mobile';
import { Alert } from 'react-native';
import { batch } from '@/src/lib/batch';
import { EDIT_SECTIONS, type ActionControl, type SliderControl } from '@/src/lib/edit-sections';
import { renderRecipe } from '@/src/lib/editor-replay';
import { errorMessage } from '@/src/lib/error-message';
import type { ProjectRecipe } from '@/src/lib/project-store';
import { adjustmentKey } from '@/src/lib/skin-adjustments';
import {
  baseIndexAt,
  emptyTimeline,
  moveCursor,
  pushEntry,
  recipeAt,
  type TimelineChange,
  type TimelineEntry,
} from '@/src/lib/timeline';
import { personSelection } from '@/src/state/person-selection';
import { adjustments, appliedActions, historyVersion } from '@/src/state/edits';
import { timeline } from '@/src/state/history';
import { historyReady } from '@/src/state/project-session';
import { currentImage, editBaseImage, originalImage, replayHidden } from '@/src/state/session';

/**
 * The picture each blemish step left, by step: the base that the edits after it are rendered on. Kept for as long as the
 * step is in the timeline, so undoing or redoing across a blemish only swaps the base instead of healing again.
 */
const blemishBases = new Map<TimelineEntry, AbraImage>();

/** The group of the action picked last, while nothing else has been done since: picking another one in it replaces that step. */
let lastGroup: string | null = null;

export const PERSON_MODEL_MISSING =
  'Edits made to a specific person need the person-detection AI model, which is not on this device. ' +
  'Download the AI models from the Storage tab, then reopen the project.';

/** Whether a recipe has an adjustment aimed at one detected person (keys like `skin-smooth:person:1`). */
export const editsAPerson = (recipe: ProjectRecipe) =>
  Object.keys(recipe.adjustments).some(key => key.includes(':person:'));

/** The picture a blemish step leaves: the edits it healed rendered on `base`, with the blemish removed. The caller owns it. */
export function healBlemish(base: AbraImage, entry: TimelineEntry): AbraImage {
  if (entry.change.type !== 'blemish' || !entry.before) throw new Error('A saved blemish step is damaged.');
  const healed = renderRecipe(base, entry.before);
  try {
    healed.removeBlemish(entry.change.x, entry.change.y, entry.change.radius);
  } catch (error) {
    healed.uniffiDestroy();
    throw error;
  }
  return healed;
}

/** Keeps `image` as the picture `entry` left. The timeline owns it from here and frees it when the step is dropped. */
export function keepBlemishBase(entry: TimelineEntry, image: AbraImage) {
  blemishBases.set(entry, image);
}

/** Frees every kept picture and empties the timeline, when the editor session ends. */
export function clearTimeline() {
  for (const image of blemishBases.values()) image.uniffiDestroy();
  blemishBases.clear();
  lastGroup = null;
  timeline.next(emptyTimeline);
}

const isInGroup = (key: string, group: string) =>
  EDIT_SECTIONS.some(section => section.controls.some(c => c.kind === 'action' && c.key === key && c.group === group));

/** Adds a step to the timeline, freeing the pictures of the undone steps it drops. Does nothing until the history is built. */
function record(entry: TimelineEntry, replaceLast = false) {
  if (!historyReady.value) return;
  const result = pushEntry(timeline.value, entry, replaceLast);
  for (const dropped of result.dropped) {
    blemishBases.get(dropped)?.uniffiDestroy();
    blemishBases.delete(dropped);
  }
  timeline.next(result.timeline);
}

/** Puts a change on the timeline, with the edit state it leads to. */
function recordEdit(
  change: TimelineChange,
  next: { adjustments?: Record<string, number>; appliedActions?: string[] },
  replaceLast = false,
) {
  record(
    {
      change,
      adjustments: next.adjustments ?? { ...adjustments.value },
      appliedActions: next.appliedActions ?? [...appliedActions.value],
    },
    replaceLast,
  );
}

/** Tap on an action control: toggles it in the applied stack, clearing the rest of its group. */
function toggleAction(control: ActionControl) {
  // Picking another option in the same single-select group as the last step replaces that step
  // instead of adding a new one — trying Warm, Cold, Sunny should leave one undo step behind.
  const replace = !!control.group && lastGroup === control.group;
  lastGroup = control.group ?? null;
  const keys = appliedActions.value;
  const on = !keys.includes(control.key);
  // Controls sharing a group are mutually exclusive: applying one clears the others in that group.
  const others = control.group ? keys.filter(key => !isInGroup(key, control.group!)) : keys;
  const next = on ? [...others, control.key] : keys.filter(key => key !== control.key);
  recordEdit({ type: 'action', key: control.key, on, group: control.group }, { appliedActions: next }, replace);
  appliedActions.next(next);
}

/** Whether the timeline ends with a step for this slider. Releasing the same slider again replaces that step, not adds one. */
const endsWithSlider = (key: string) => {
  const { entries, cursor } = timeline.value;
  const last = cursor > 0 && cursor === entries.length ? entries[cursor - 1].change : undefined;
  return last?.type === 'slider' && last.key === key;
};

/** A slider was released at `value`. */
function commitSlider(control: SliderControl, value: number) {
  replayHidden.next(!!(control.live || control.tool));
  lastGroup = null;
  const key = adjustmentKey(control.key, personSelection.value.selectedId);
  const next = { ...adjustments.value, [key]: value };
  recordEdit({ type: 'slider', key, value }, { adjustments: next }, endsWithSlider(key));
  adjustments.next(next);
}

/** A slider's effect was turned off: it has no value again, so it is not part of the stack. Does nothing if it was already off. */
function removeSlider(control: SliderControl) {
  const key = adjustmentKey(control.key, personSelection.value.selectedId);
  if (adjustments.value[key] === undefined) return;
  replayHidden.next(!!(control.live || control.tool));
  lastGroup = null;
  const { [key]: _removed, ...rest } = adjustments.value;
  recordEdit({ type: 'slider', key, value: null }, { adjustments: rest }, endsWithSlider(key));
  adjustments.next(rest);
}

/**
 * The blemish tool healed `currentImage` at this spot. That is a step of its own, and the healed picture becomes the base
 * that the edits after it are rendered on, so those start empty.
 */
function commitBlemish(blemish: { x: number; y: number; radius: number }) {
  const image = currentImage.value;
  if (!image || !historyReady.value) return;
  const entry: TimelineEntry = {
    change: { type: 'blemish', ...blemish },
    adjustments: {},
    appliedActions: [],
    before: { adjustments: { ...adjustments.value }, appliedActions: [...appliedActions.value] },
  };
  record(entry);
  blemishBases.set(entry, image.copy() as AbraImage);
  editBaseImage.value?.uniffiDestroy();
  editBaseImage.next(image.copy() as AbraImage);
  lastGroup = null;
  adjustments.next({});
  appliedActions.next([]);
}

/** Moves to `cursor`: shows the edit state of that step, on the base picture it was made on. */
function moveTo(cursor: number) {
  const before = timeline.value;
  const after = moveCursor(before, cursor);
  if (after.cursor === before.cursor) return;

  // Crossing a blemish changes the base. Setting the edit state below makes the replay render it on that base.
  const baseIndex = baseIndexAt(after.entries, after.cursor);
  let source: AbraImage | null | undefined;
  if (baseIndex !== baseIndexAt(before.entries, before.cursor)) {
    source = baseIndex < 0 ? originalImage.value : blemishBases.get(after.entries[baseIndex]);
  }
  try {
    // A saved project only rebuilds the pictures up to where it was closed. The ones after that, which redo brings back, are
    // healed the first time they are needed, from the picture before them.
    if (!source && baseIndex >= 0 && baseIndex !== baseIndexAt(before.entries, before.cursor)) {
      const earlier = baseIndexAt(after.entries, baseIndex);
      const previous = earlier < 0 ? originalImage.value : blemishBases.get(after.entries[earlier]);
      if (!previous) throw new Error('The picture before this blemish is missing.');
      source = healBlemish(previous, after.entries[baseIndex]);
      blemishBases.set(after.entries[baseIndex], source);
    }
    // The person model is needed to render an edit made to one person. Say so, rather than fail inside the replay.
    const step = after.cursor > before.cursor ? after.entries[after.cursor - 1] : undefined;
    if (step && !personDetectionDownloaded() && editsAPerson(step.before ?? step)) throw new Error(PERSON_MODEL_MISSING);
  } catch (error) {
    Alert.alert('Could not redo', errorMessage(error));
    return;
  }
  lastGroup = null;
  timeline.next(after);
  if (source) {
    editBaseImage.value?.uniffiDestroy();
    editBaseImage.next(source.copy() as AbraImage);
  }
  const recipe = recipeAt(after.entries, after.cursor);
  adjustments.next(recipe.adjustments);
  appliedActions.next(recipe.appliedActions);
  historyVersion.next(historyVersion.value + 1);
}

export const undoHistory = () => batch(() => moveTo(timeline.value.cursor - 1));

export const redoHistory = () => batch(() => moveTo(timeline.value.cursor + 1));

// Each of these emits several subjects; batching turns that into a single React render.
const batched =
  <A extends unknown[]>(fn: (...args: A) => void) =>
  (...args: A) =>
    batch(() => fn(...args));
const toggleActionBatched = batched(toggleAction);
export { toggleActionBatched as toggleAction };
const commitSliderBatched = batched(commitSlider);
export { commitSliderBatched as commitSlider };
const removeSliderBatched = batched(removeSlider);
export { removeSliderBatched as removeSlider };
const commitBlemishBatched = batched(commitBlemish);
export { commitBlemishBatched as commitBlemish };
