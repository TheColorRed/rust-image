import type { TimelineEntry } from '@/src/lib/timeline';
import { NativeModules, Platform } from 'react-native';

/**
 * The edit state at one moment: the slider values and the applied actions. A blemish removal, if the checkpoint ended
 * with one, is a tap at `x`, `y` (image pixels) healed with `radius`.
 */
export interface ProjectRecipe {
  adjustments: Record<string, number>;
  appliedActions: string[];
}

/**
 * One saved project: a folder in the app's files folder (kept until the user deletes it, unlike the cache) holding
 * the untouched original, `source.<ext>`, the final image, `resultFile`, and `project.json`, which is this object
 * without `dir`.
 */
export interface Project {
  /** Bumped when the saved shape changes, so older projects can still be read. */
  version: 1;
  id: string;
  name: string;
  /** The original's file name inside the project folder. */
  sourceFile: string;
  /** The final image's file name, once the project has been edited. It is shown before the edits are replayed. */
  resultFile?: string;
  /**
   * The skin mask the AI model made for the original, saved as a lossless gray WebP. Opening the project loads it instead of running
   * the model, so the edits replay with the same mask (even when the model is not on this device) and open faster.
   */
  skinMaskFile?: string;
  /** Every step, in the order it was performed, including ones that were undone and can be redone. */
  timeline: TimelineEntry[];
  /** How many steps of `timeline` are applied: where undo had got to when the project was closed. */
  cursor: number;
  createdAt: number;
  modifiedAt: number;
  /** The project folder as a `file://` URI. Filled in when the project is read, never saved. */
  dir: string;
}

type ProjectStoreModule = {
  createProject(sourceUri: string, fileName: string): Promise<string>;
  listProjects(): Promise<string[]>;
  writeProject(id: string, json: string): Promise<void>;
  deleteProject(id: string): Promise<boolean>;
};

const store = NativeModules.ProjectStore as ProjectStoreModule | undefined;

function native(): ProjectStoreModule {
  if (Platform.OS !== 'android' || !store?.createProject) {
    throw new Error('The installed app is older than this code. Rebuild it with npm run dev:android.');
  }
  return store;
}

/** The original image of a project, as a `file://` URI. */
export const projectSourceUri = (project: Project) => `${project.dir}/${project.sourceFile}`;

/** What to show for a project: its final image if it has been edited, otherwise the original. */
export const projectImageUri = (project: Project) => `${project.dir}/${project.resultFile ?? project.sourceFile}`;

/** Copies an image (`content://` or `file://`) into a new project folder. */
export async function createProject(sourceUri: string, fileName: string): Promise<Project> {
  return JSON.parse(await native().createProject(sourceUri, fileName));
}

/** Every saved project, newest first. */
export async function listProjects(): Promise<Project[]> {
  return (await native().listProjects()).map(json => JSON.parse(json));
}

/** Saves the project's name, edit recipe and final image's name. `dir` is not saved; the source image is never rewritten. */
export async function writeProject({ dir: _dir, ...project }: Project): Promise<void> {
  return native().writeProject(project.id, JSON.stringify(project));
}

/** Deletes the project's folder. The photo it was made from is never touched. */
export async function deleteProject(id: string): Promise<boolean> {
  return native().deleteProject(id);
}
