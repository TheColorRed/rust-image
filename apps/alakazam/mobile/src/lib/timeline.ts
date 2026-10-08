import type { ProjectRecipe } from '@/src/lib/project-store';

/** What an entry changed: for showing the history, and to tell whether a repeat edit should replace it. */
export type TimelineChange =
  | { type: 'slider'; key: string; /** `null` when the slider was turned off. */ value: number | null }
  | { type: 'action'; key: string; on: boolean; group?: string }
  | { type: 'blemish'; x: number; y: number; radius: number }
  /** Edits made while a project was still loading, merged into its saved edits as one step. */
  | { type: 'pending' };

/**
 * One step of the edit history, in the order it was performed. `adjustments` and `appliedActions` are the edit state after
 * the step, on top of the picture the last blemish left (or the original, when there is none).
 */
export interface TimelineEntry extends ProjectRecipe {
  change: TimelineChange;
  /** Blemish steps only: the edit state that was healed. Afterwards the edit state starts empty on the healed picture. */
  before?: ProjectRecipe;
}

/**
 * The whole history: every step, and how many of them are applied. Undo and redo only move `cursor`; a new step drops the
 * undone ones after it. Only `entries.slice(0, cursor)` is what was performed, aside from the undos.
 */
export interface Timeline {
  entries: TimelineEntry[];
  cursor: number;
}

export const emptyTimeline: Timeline = { entries: [], cursor: 0 };

export const emptyRecipe = (): ProjectRecipe => ({ adjustments: {}, appliedActions: [] });

/** The edit state after the first `cursor` steps, which is what the controls show and the picture is rendered from. */
export function recipeAt(entries: readonly TimelineEntry[], cursor: number): ProjectRecipe {
  const last = cursor > 0 ? entries[cursor - 1] : undefined;
  return last
    ? { adjustments: { ...last.adjustments }, appliedActions: [...last.appliedActions] }
    : emptyRecipe();
}

/** Index of the last blemish among the first `cursor` steps, or -1 when the picture is built from the original. */
export function baseIndexAt(entries: readonly TimelineEntry[], cursor: number): number {
  for (let index = Math.min(cursor, entries.length) - 1; index >= 0; index--) {
    if (entries[index].change.type === 'blemish') return index;
  }
  return -1;
}

/**
 * Adds a step after the cursor, dropping the undone steps after it. With `replaceLast` it takes the place of the step just
 * before the cursor instead, which only applies when that step is the last one. Returns the new timeline and the steps it
 * dropped, so whatever they hold can be freed.
 */
export function pushEntry(
  timeline: Timeline,
  entry: TimelineEntry,
  replaceLast = false,
): { timeline: Timeline; dropped: TimelineEntry[] } {
  const replace = replaceLast && timeline.cursor > 0 && timeline.cursor === timeline.entries.length;
  const keep = replace ? timeline.cursor - 1 : timeline.cursor;
  return {
    timeline: { entries: [...timeline.entries.slice(0, keep), entry], cursor: keep + 1 },
    dropped: timeline.entries.slice(keep),
  };
}

/** The same timeline with the cursor moved, kept between the first step and the last. */
export function moveCursor(timeline: Timeline, cursor: number): Timeline {
  return { entries: timeline.entries, cursor: Math.max(0, Math.min(cursor, timeline.entries.length)) };
}

export const canUndo = (timeline: Timeline) => timeline.cursor > 0;
export const canRedo = (timeline: Timeline) => timeline.cursor < timeline.entries.length;
