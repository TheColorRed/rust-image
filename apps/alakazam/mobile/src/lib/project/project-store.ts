import type { AdjustmentValue } from '@/src/lib/project/edit-sections';
import type { TimelineEntry } from '@/src/lib/project/timeline';
import { NativeModules, Platform } from 'react-native';

/**
 * The edit state at one moment: the slider values and the applied actions. A blemish removal, if the checkpoint ended
 * with one, is a tap at `x`, `y` (image pixels) healed with `radius`.
 */
export interface ProjectRecipe {
  /** A number for a slider control with one slider (as saved before controls could have several), an array for several. */
  adjustments: Record<string, AdjustmentValue>;
  appliedActions: string[];
}

export interface SavedPersonDetection {
  id: number;
  x: number;
  y: number;
  width: number;
  height: number;
  confidence: number;
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
   * Every mask the AI models made for the original, one lossless gray WebP each, by name (see `saved-masks.ts`). Opening the project
   * loads them instead of running the models, so the edits replay with the same masks, even when the models are not on this device.
   */
  masks?: { name: string; file: string }[];
  /** The groups of masks (`makeup`, `hair`, `subject`) whose models have run, including ones that found nothing. */
  maskGroups?: string[];
  /** Metadata for the people, whose masks are in `masks` as `person:<id>`. An empty array means detection ran and found nobody. */
  people?: SavedPersonDetection[];
  /**
   * The square around each person's face, by the person's id, in the pixels the people were found in. Opening the project loads them
   * instead of running the face models. An empty object means the models ran and found no face.
   */
  faceBoxes?: Record<number, { x: number; y: number; width: number; height: number }>;
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
