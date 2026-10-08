import { BehaviorSubject } from 'rxjs';
import { map } from 'rxjs/operators';
import { canRedo, canUndo, emptyTimeline, type Timeline } from '@/src/lib/timeline';

/**
 * The edit history: every step in the order it was performed, and how many are applied. Undo and redo move the cursor.
 * It is empty until the editor session is ready, which is why the undo and redo buttons start out disabled.
 */
export const timeline = new BehaviorSubject<Timeline>(emptyTimeline);
export const timeline$ = timeline.asObservable();

// The buttons follow the timeline itself, so they cannot fall out of step with it.
export const canUndo$ = timeline$.pipe(map(canUndo));
export const canRedo$ = timeline$.pipe(map(canRedo));
