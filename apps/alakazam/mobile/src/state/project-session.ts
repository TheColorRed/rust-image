import { type Project } from '@/src/lib/project-store';
import { BehaviorSubject } from 'rxjs';

/** The project open in the editor, or null when the editor was opened on a photo that is not a project. */
export const openProject = new BehaviorSubject<Project | null>(null);

/**
 * Whether the editor's edit history is built. A project shows its saved final image and takes edits at once, but those
 * edits are only pending work until the pictures its blemish steps left have been rebuilt; recording steps, the blemish
 * tool and saving the project wait for this.
 */
export const historyReady = new BehaviorSubject(false);
export const historyReady$ = historyReady.asObservable();
