import { type Project } from '@/src/lib/project/project-store';
import { BehaviorSubject } from 'rxjs';

/** The project open in the editor, or null when the editor was opened on a photo that is not a project. */
export const openProject = new BehaviorSubject<Project | null>(null);

/** Whether the editor has loaded its baked base and saved control snapshots so tools can edit the document. */
export const historyReady = new BehaviorSubject(false);
export const historyReady$ = historyReady.asObservable();
