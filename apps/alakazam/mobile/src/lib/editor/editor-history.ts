import { batch } from '@/src/lib/utils/batch';
import { removeBlemish } from '@/src/lib/beauty/blemish';
import {
  EDIT_SECTIONS,
  leafControls,
  type ActionControl,
  type AdjustmentValue,
  type SliderControl,
} from '@/src/lib/project/edit-sections';
import { renderRecipe } from '@/src/lib/editor/editor-replay';
import { editorPreview } from '@/src/lib/editor/editor-preview';
import { errorMessage } from '@/src/lib/utils/error-message';
import { type CropRect, cropPicture } from '@/src/lib/tools/crop';
import type { ProjectRecipe } from '@/src/lib/project/project-store';
import { type EditPreset } from '@/src/lib/project/presets';
import { applyRedEyeStep } from '@/src/lib/beauty/red-eye';
import { adjustmentKey } from '@/src/lib/beauty/skin-adjustments';
import {
  baseIndexAt,
  changesPixels,
  emptyTimeline,
  moveCursor,
  pushEntry,
  recipeAt,
  type PixelChange,
  type TimelineChange,
  type TimelineEntry,
} from '@/src/lib/project/timeline';
import { adjustments, appliedActions, areaOutlinesHidden, focusedControlKey, historyVersion } from '@/src/state/edits';
import { timeline } from '@/src/state/history';
import { personSelection } from '@/src/state/person-selection';
import { historyReady, openProject } from '@/src/state/project-session';
import { currentImage, editBaseImage, originalImage, replayHidden } from '@/src/state/session';
import { AbraImage, personDetectionDownloaded } from '@alakazam/mobile';
import { Alert } from 'react-native';
import { release } from '@/src/lib/native/native-handles';

/**
 * The picture each pixel-changing step left (a blemish, red-eye, or crop), by step: the base that the edits after it are rendered on.
 * Kept for as long as the step is in the timeline, so undoing or redoing across it only swaps the base instead of redoing it.
 */
const blemishBases = new Map<TimelineEntry, AbraImage>();


/** The group of the action picked last, while nothing else has been done since: picking another one in it replaces that step. */
let lastGroup: string | null = null;

export const PERSON_MODEL_MISSING =
  'Edits made to a specific person need the person-detection AI model, which is not on this device. ' +
  'Download the AI models from the Plugins tab, then reopen the project.';

/** Whether a recipe has an adjustment aimed at one detected person (keys like `skin-smooth:person:1`). */
export const editsAPerson = (recipe: ProjectRecipe) =>
  Object.keys(recipe.adjustments).some(key => key.includes(':person:'));

/**
 * The picture a step that changes pixels leaves: the edits it baked in rendered on `base`, with the blemish removed or the
 * red-eye fixed. The caller owns it. A step that removed objects has no way to be made again here, so it fails.
 */
export function healBlemish(base: AbraImage, entry: TimelineEntry): AbraImage {
  const change = entry.change;
  if (!changesPixels(change) || !entry.before) {
    throw new Error('A saved retouch step is damaged.');
  }
  // The model that removes objects runs on a worker thread and cannot be waited for here, so its picture is only ever made once,
  // when the tool is used, and kept (see `commitRetouch`). A saved project keeps it as the step's checkpoint.
  if (change.type === 'object-removal') throw new Error('The picture after removing objects is missing, and cannot be made again.');
  const healed = renderRecipe(base, entry.before);
  if (change.type === 'crop') {
    try {
      return cropPicture(healed, change, change.turn);
    } finally {
      release(healed);
    }
  }
  try {
    if (change.type === 'blemish') removeBlemish(healed, change.x, change.y, change.radius);
    else applyRedEyeStep(healed, change);
  } catch (error) {
    release(healed);
    throw error;
  }
  return healed;
}

/** Keeps `image` as the picture `entry` left. The timeline owns it from here and frees it when the step is dropped. */
export function keepBlemishBase(entry: TimelineEntry, image: AbraImage) {
  blemishBases.set(entry, image);
}

/** The rendered checkpoint owned by a pixel-changing history step, when it has been built. */
export const checkpointFor = (entry: TimelineEntry): AbraImage | undefined => blemishBases.get(entry);

/** Frees every kept picture and empties the timeline, when the editor session ends. */
export function clearTimeline() {
  for (const image of blemishBases.values()) release(image);
  blemishBases.clear();
  lastGroup = null;
  timeline.next(emptyTimeline);
}

const isInGroup = (key: string, group: string) =>
  EDIT_SECTIONS.some(section =>
    leafControls(section.controls).some(c => c.kind === 'action' && c.key === key && c.group === group),
  );

/** Adds a step to the timeline, freeing the pictures of the undone steps it drops. Does nothing until the history is built. */
function record(entry: TimelineEntry, replaceLast = false) {
  if (!historyReady.value) return;
  const result = pushEntry(timeline.value, entry, replaceLast);
  for (const dropped of result.dropped) {
    release(blemishBases.get(dropped));
    blemishBases.delete(dropped);
  }
  timeline.next(result.timeline);
}

/** Puts a change on the timeline, with the edit state it leads to. */
function recordEdit(
  change: TimelineChange,
  next: { adjustments?: Record<string, AdjustmentValue>; appliedActions?: string[] },
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

/** Tap on an action control: toggles it in the applied stack, clearing the rest of its group. A repeatable one adds another step each tap. */
function toggleAction(control: ActionControl) {
  // Picking another option in the same single-select group as the last step replaces that step
  // instead of adding a new one — trying Warm, Cold, Sunny should leave one undo step behind.
  const replace = !!control.group && lastGroup === control.group;
  lastGroup = control.group ?? null;
  const keys = appliedActions.value;
  if (control.repeatable) {
    const next = [...keys, control.key];
    recordEdit({ type: 'action', key: control.key, on: true, group: control.group }, { appliedActions: next });
    appliedActions.next(next);
    return;
  }
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
function commitSlider(control: SliderControl, value: AdjustmentValue) {
  replayHidden.next(!!(control.tool || control.joint || control.layer));
  areaOutlinesHidden.next(false);
  lastGroup = null;
  const key = adjustmentKey(control.key, personSelection.value.selectedId);
  const next = { ...adjustments.value, [key]: value };
  recordEdit({ type: 'slider', key, value }, { adjustments: next }, endsWithSlider(key));
  adjustments.next(next);
}

/** A slider's effect was turned off: it has no value again, so it is not part of the stack. Does nothing if it was already off. */
function removeSlider(control: SliderControl) {
  // A look put on everyone is stored under the plain key and one for a person under theirs, and both can be there at once.
  // Both go, or the one left behind keeps the control showing as applied.
  const personKey = adjustmentKey(control.key, personSelection.value.selectedId);
  const key = adjustments.value[personKey] === undefined ? control.key : personKey;
  const hasArea = control.areaKeys?.some(areaKey => adjustments.value[areaKey] !== undefined) ?? false;
  // An area can make a control apply even when its default slider value was never saved.
  if (adjustments.value[key] === undefined && !hasArea) return;
  replayHidden.next(!!(control.tool || control.joint || control.layer));
  lastGroup = null;
  // Some controls keep their masks as separate adjustments. Removing the effect must remove those
  // too, or they keep the control in the stack and constrain it if it is applied again.
  const removed = new Set([key, control.key, ...(control.areaKeys ?? [])]);
  const rest = Object.fromEntries(Object.entries(adjustments.value).filter(([savedKey]) => !removed.has(savedKey)));
  recordEdit({ type: 'slider', key, value: null }, { adjustments: rest }, endsWithSlider(key));
  adjustments.next(rest);
}

/** A color's drawn area changed. `points` is a flat list of x, y fractions of the photo, or null to clear it. */
function commitColorArea(key: string, points: number[] | null) {
  replayHidden.next(false);
  // The committed replay below the live slider renderer has the new area, while that renderer still holds the previous frame.
  // Hide it so the refreshed committed image is visible immediately; the next slider move will publish a new live frame.
  editorPreview.hide();
  lastGroup = null;
  const next = { ...adjustments.value };
  if (points) next[key] = points;
  else delete next[key];
  recordEdit({ type: 'slider', key, value: points }, { adjustments: next }, endsWithSlider(key));
  adjustments.next(next);
}

/** Applies the checked parts of a preset as one undoable change, preserving every unchecked existing edit. */
export function applyPreset(preset: EditPreset, selectedKeys = new Set([...Object.keys(preset.adjustments), ...preset.appliedActions])) {
  const keys = selectedKeys;
  const nextAdjustments = {
    ...Object.fromEntries(Object.entries(adjustments.value).filter(([key]) => !keys.has(key))),
    ...Object.fromEntries(Object.entries(preset.adjustments).filter(([key]) => keys.has(key))),
  };
  const selectedActions = preset.appliedActions.filter(key => keys.has(key));
  const groups = new Set(selectedActions.flatMap(key => {
    for (const section of EDIT_SECTIONS) {
      const control = leafControls(section.controls).find(item => item.kind === 'action' && item.key === key);
      if (control?.kind === 'action' && control.group) return [control.group];
    }
    return [];
  }));
  const nextActions = [
    ...appliedActions.value.filter(key => {
      if (keys.has(key)) return false;
      for (const section of EDIT_SECTIONS) {
        const control = leafControls(section.controls).find(item => item.kind === 'action' && item.key === key);
        if (control?.kind === 'action' && control.group && groups.has(control.group)) return false;
      }
      return true;
    }),
    ...selectedActions,
  ];
  replayHidden.next(true);
  lastGroup = null;
  recordEdit({ type: 'preset', id: preset.id }, { adjustments: nextAdjustments, appliedActions: nextActions });
  batch(() => {
    adjustments.next(nextAdjustments);
    appliedActions.next(nextActions);
  });
}

/**
 * A tool changed the pixels of `currentImage`: it healed a blemish or fixed red-eye. That is a step of its own, and the new
 * picture becomes the base that the edits after it are rendered on, so those start empty.
 */
function commitRetouch(change: PixelChange) {
  const image = currentImage.value;
  if (!image || !historyReady.value) return;
  const entry: TimelineEntry = {
    change,
    adjustments: {},
    appliedActions: [],
    before: { adjustments: { ...adjustments.value }, appliedActions: [...appliedActions.value] },
  };
  blemishBases.set(entry, image.clone() as AbraImage);
  record(entry);
  release(editBaseImage.value);
  editBaseImage.next(image.clone() as AbraImage);
  lastGroup = null;
  adjustments.next({});
  appliedActions.next([]);
}

/**
 * The crop tool cut the photo to `rect`, turned by `turn` radians. Unlike a retouch, the picture changes size, so the full-size photo is replaced too.
 * The cropped photo is the base the edits after it are rendered on, so those start empty.
 */
function commitCrop(rect: CropRect, turn = 0) {
  const image = currentImage.value;
  if (!image || !historyReady.value) return;
  const cropped = cropPicture(image, rect, turn);
  const change: TimelineChange = {
    type: 'crop',
    left: rect.left,
    top: rect.top,
    right: rect.right,
    bottom: rect.bottom,
    ...(turn ? { turn } : {}),
  };
  const entry: TimelineEntry = {
    change,
    adjustments: {},
    appliedActions: [],
    before: { adjustments: { ...adjustments.value }, appliedActions: [...appliedActions.value] },
  };
  record(entry);
  blemishBases.set(entry, cropped.clone() as AbraImage);
  release(editBaseImage.value);
  editBaseImage.next(cropped.clone() as AbraImage);
  release(image);
  currentImage.next(cropped);
  lastGroup = null;
  adjustments.next({});
  appliedActions.next([]);
}

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
    if (step && !personDetectionDownloaded() && !openProject.value?.people && editsAPerson(step.before ?? step))
      throw new Error(PERSON_MODEL_MISSING);
  } catch (error) {
    Alert.alert('Could not redo', errorMessage(error));
    return;
  }
  lastGroup = null;
  timeline.next(after);
  if (source) {
    release(editBaseImage.value);
    editBaseImage.next(source.clone() as AbraImage);
  }
  const recipe = recipeAt(after.entries, after.cursor);
  adjustments.next(recipe.adjustments);
  appliedActions.next(recipe.appliedActions);
  historyVersion.next(historyVersion.value + 1);
}

/**
 * Puts the photo back as it was opened: no edits, no blemish fixes, and an empty history. It leaves no step behind, so it
 * cannot be undone.
 */
export function resetEdits() {
  const original = originalImage.value;
  if (!original) return;
  batch(() => {
    clearTimeline();
    release(editBaseImage.value);
    editBaseImage.next(original.clone() as AbraImage);
    focusedControlKey.next(null);
    replayHidden.next(false);
    // The replay follows these, so the picture is drawn again from the original.
    adjustments.next({});
    appliedActions.next([]);
    historyVersion.next(historyVersion.value + 1);
  });
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
const commitColorAreaBatched = batched(commitColorArea);
export { commitColorAreaBatched as commitColorArea };
const commitRetouchBatched = batched(commitRetouch);
export { commitRetouchBatched as commitRetouch };
const commitBlemishBatched = batched((blemish: { x: number; y: number; radius: number }) =>
  commitRetouch({ type: 'blemish', ...blemish }),
);
export { commitBlemishBatched as commitBlemish };
const commitCropBatched = batched(commitCrop);
export { commitCropBatched as commitCrop };
