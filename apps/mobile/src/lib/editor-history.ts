import { AbraImage } from '@alakazam/mobile';
import { batch } from '@/src/lib/batch';
import { EDIT_SECTIONS, type ActionControl, type SliderControl } from '@/src/lib/edit-sections';
import { adjustments, appliedActions, resetVersion } from '@/src/state/edits';
import {
  canRedo,
  canRedoDraft,
  canUndo,
  canUndoDraft,
  draftUiHistory,
  lastDraftGroup,
  recordDraft,
  type DraftUiState,
} from '@/src/state/history';
import { previewSourceImage } from '@/src/state/preview';
import {
  checkpointHistory,
  currentImage,
  draftHistory,
  editBaseImage,
  originalImage,
  replayHidden,
  saved,
} from '@/src/state/session';

const freshDraftUi = () => ({ entries: [{ adjustments: {}, appliedActions: [] }], index: 0 });

/** Whether a draft is in progress (any uncommitted step on top of the last checkpoint). */
export const isDraftDirty = () => draftHistory.value?.canUndo() ?? false;

/**
 * The image a held before/after preview should show. While a draft is in progress "before" is the
 * last checkpoint (what's actually saved); with no draft the current image already *is* the last
 * checkpoint, so it falls back to the untouched original — otherwise holding would show no change.
 */
export const comparisonImage = () => (isDraftDirty() ? editBaseImage.value : originalImage.value);

/** Re-derives every undo/redo flag from the two histories. */
export function syncHistoryFlags() {
  const dirty = isDraftDirty();
  canUndoDraft.next(dirty);
  canRedoDraft.next(draftHistory.value?.canRedo() ?? false);
  canUndo.next(!dirty && (checkpointHistory.value?.canUndo() ?? false));
  canRedo.next(!dirty && (checkpointHistory.value?.canRedo() ?? false));
}

const isInGroup = (key: string, group: string) =>
  EDIT_SECTIONS.some(section => section.controls.some(c => c.kind === 'action' && c.key === key && c.group === group));

/** Tap on an action control: toggles it in the applied stack, clearing the rest of its group. */
function toggleAction(control: ActionControl) {
  saved.next(false);
  // Picking another option in the same single-select group as the last draft step replaces that
  // step instead of adding a new one — trying Warm, Cold, Sunny should leave one undo step behind.
  recordDraft.next(control.group && lastDraftGroup.value === control.group ? 'replace' : 'push');
  lastDraftGroup.next(control.group ?? null);
  const keys = appliedActions.value;
  if (keys.includes(control.key)) {
    appliedActions.next(keys.filter(key => key !== control.key));
    return;
  }
  // Controls sharing a group are mutually exclusive: applying one clears the others in that group.
  const others = control.group ? keys.filter(key => !isInGroup(key, control.group!)) : keys;
  appliedActions.next([...others, control.key]);
}

/** A slider was released at `value`. */
function commitSlider(control: SliderControl, value: number) {
  replayHidden.next(!!control.live);
  saved.next(false);
  recordDraft.next('push');
  lastDraftGroup.next(null);
  adjustments.next({ ...adjustments.value, [control.key]: value });
}

/** A slider's effect was turned off: it has no value again, so it is not part of the stack. Does nothing if it was already off. */
function removeSlider(control: SliderControl) {
  if (adjustments.value[control.key] === undefined) return;
  replayHidden.next(!!control.live);
  saved.next(false);
  recordDraft.next('push');
  lastDraftGroup.next(null);
  const { [control.key]: _removed, ...rest } = adjustments.value;
  adjustments.next(rest);
}

/**
 * Called by the replay once it has rendered `next` for the pending edit: records it as a new
 * draft step, or swaps it in for the previous one when a same-group option was picked.
 */
export function recordDraftStep(next: AbraImage) {
  const mode = recordDraft.value;
  if (mode === 'none') return;
  const step: DraftUiState = { adjustments: { ...adjustments.value }, appliedActions: [...appliedActions.value] };
  const ui = draftUiHistory.value;
  if (mode === 'replace') {
    // Undo then push: the history has no "replace", but stepping the cursor back and pushing
    // discards the redo branch — exactly the entry being replaced — before adding `next` there.
    draftHistory.value?.undo();
    draftHistory.value?.push(next);
    const entries = [...ui.entries];
    entries[ui.index] = step;
    draftUiHistory.next({ entries, index: ui.index });
  } else {
    draftHistory.value?.push(next);
    const entries = [...ui.entries.slice(0, ui.index + 1), step];
    draftUiHistory.next({ entries, index: entries.length - 1 });
  }
  recordDraft.next('none');
  syncHistoryFlags();
}

/** Throws away the draft and goes back to the last checkpoint. */
function resetDraft() {
  const base = editBaseImage.value;
  if (base) {
    draftHistory.value?.reset(base);
    draftUiHistory.next(freshDraftUi());
    currentImage.value?.uniffiDestroy();
    const image = base.copy() as AbraImage;
    currentImage.next(image);
    previewSourceImage.next(image);
  }
  lastDraftGroup.next(null);
  adjustments.next({});
  appliedActions.next([]);
  resetVersion.next(resetVersion.value + 1);
  syncHistoryFlags();
  saved.next(false);
}

/** Commits the draft as a new checkpoint. */
function saveDraft() {
  const image = currentImage.value;
  if (!image) return;
  checkpointHistory.value?.push(image);
  draftHistory.value?.reset(image);
  draftUiHistory.next(freshDraftUi());
  editBaseImage.value?.uniffiDestroy();
  editBaseImage.next(image.copy() as AbraImage);
  lastDraftGroup.next(null);
  adjustments.next({});
  appliedActions.next([]);
  syncHistoryFlags();
  saved.next(true);
}

function restoreImage(next: AbraImage, state?: DraftUiState, replaceEditBase = true) {
  currentImage.value?.uniffiDestroy();
  currentImage.next(next);
  if (replaceEditBase) {
    editBaseImage.value?.uniffiDestroy();
    editBaseImage.next(next.copy() as AbraImage);
  }
  // Navigating history ends any in-progress "trying options in this group" streak.
  lastDraftGroup.next(null);
  adjustments.next(state?.adjustments ?? {});
  appliedActions.next(state?.appliedActions ?? []);
  saved.next(false);
  previewSourceImage.next(next);
}

function stepDraft(direction: -1 | 1) {
  const history = draftHistory.value;
  const next = (direction < 0 ? history?.undo() : history?.redo()) as AbraImage | undefined;
  if (!next) return;
  const ui = draftUiHistory.value;
  const index = ui.index + direction;
  draftUiHistory.next({ entries: ui.entries, index });
  restoreImage(next, ui.entries[index], false);
  syncHistoryFlags();
}

function stepCheckpoint(direction: -1 | 1) {
  if (isDraftDirty()) return;
  const history = checkpointHistory.value;
  const next = (direction < 0 ? history?.undo() : history?.redo()) as AbraImage | undefined;
  if (!next) return;
  draftHistory.value?.reset(next);
  draftUiHistory.next(freshDraftUi());
  restoreImage(next);
  syncHistoryFlags();
}

/** Undo steps the draft while one is in progress, and the saved checkpoints otherwise. */
export const undoHistory = () => batch(() => (isDraftDirty() ? stepDraft(-1) : stepCheckpoint(-1)));

export const redoHistory = () => batch(() => (draftHistory.value?.canRedo() ? stepDraft(1) : stepCheckpoint(1)));

// Each of these emits several subjects; batching turns that into a single React render.
const batched = <A extends unknown[]>(fn: (...args: A) => void) => (...args: A) => batch(() => fn(...args));
const toggleActionBatched = batched(toggleAction);
export { toggleActionBatched as toggleAction };
const commitSliderBatched = batched(commitSlider);
export { commitSliderBatched as commitSlider };
const removeSliderBatched = batched(removeSlider);
export { removeSliderBatched as removeSlider };
const resetDraftBatched = batched(resetDraft);
export { resetDraftBatched as resetDraft };
const saveDraftBatched = batched(saveDraft);
export { saveDraftBatched as saveDraft };
