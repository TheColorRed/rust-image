import type { AdjustmentValue } from '@/src/lib/project/edit-sections';
import { NativeModules, Platform } from 'react-native';

/**
 * The editable slider values and applied actions currently used to render a project's control stack.
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
  /** Version 2 stores a saved document stack and no session undo history. */
  version: 2;
  id: string;
  name: string;
  /** The original's file name inside the project folder. */
  sourceFile: string;
  /** The flattened result shown in project lists and while saved control images load. */
  resultFile?: string;
  /** The current control settings. Session-only undo/redo state is not part of a project's editable state. */
  recipe: ProjectRecipe;
  /** Per-control flattened pixels, used to restore the render cache without rerunning the controls on open. */
  layers: { signature: string; file: string }[];
  /** The baked base after crop and pixel-changing tools, separate from the editable controls. */
  baseFile?: string;
  /** Crops applied to the source frame, retained so saved masks and the before view keep the correct framing. */
  savedCrops: { left: number; top: number; right: number; bottom: number; turn?: number }[];
  /**
   * Every mask the AI models made for the original, one lossless gray WebP each, by name (see `saved-masks.ts`). Opening the project
   * loads them instead of running the models, so editable controls use the same masks even when the models are not on this device.
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
