import { Mask, type MaskLike } from '@abra/core/mask';
import { type Image as AbraPixels } from '@abra/core/image';
import { personDetectionDownloaded, readImageAsync, type AbraImage, type PersonDetection } from '@alakazam/mobile';
import { makeupMasks, hairMasks } from '@/src/lib/beauty/eye-makeup';
import { subjectMask } from '@/src/lib/ai/subject';
import { toLocalPath } from '@/src/lib/editor/edit-source';
import { analysisOf, buildAnalysis, provideAnalysis } from '@/src/lib/editor/editor-preloads';
import { detectPeople, detectSkin } from '@/src/lib/ai/analysis';
import { findFaceBoxes, unloadModelsWhenIdle } from '@/src/lib/ai/faces';
import { errorMessage } from '@/src/lib/utils/error-message';
import { ownedImage } from '@/src/lib/native/native-image';
import type { Project } from '@/src/lib/project/project-store';
import { faceBoxes, people, skinMask } from '@/src/state/analysis';
import { historyReady, openProject } from '@/src/state/project-session';
import { updateProject } from '@/src/state/projects';

// Every mask an AI model makes for the original is kept in one list on the project (`Project.masks`), one lossless gray WebP each, so
// a project opens and replays with no model run. A mask has one name, and the name is the whole of how it is found: `skin`,
// `person:<id>`, and `<group>:<name>` for the masks a group of controls shares. To save a new group, add one line to `GROUPS`.

/** The groups of masks that a preload makes, by the name their masks start with. */
const GROUPS = { makeup: makeupMasks, hair: hairMasks, subject: subjectMask } as const;

/** Saves a mask as a gray image. */
const writeMask = (mask: MaskLike, file: string) => {
  const image = mask.toImage();
  try {
    image.write(file, undefined);
  } finally {
    (image as AbraPixels).uniffiDestroy();
  }
};

const destroy = (mask: MaskLike) => (mask as unknown as AbraPixels).uniffiDestroy();

/** The project without the masks of one group (`makeup`), so the group is made again and saved over, like `rescanFaceMasks`. */
export const withoutGroup = (project: Project, group: keyof typeof GROUPS): Project => ({
  ...project,
  masks: project.masks?.filter(({ name }) => !name.startsWith(`${group}:`)),
  maskGroups: project.maskGroups?.filter(saved => saved !== group),
});

/** The project without the skin, the people and their faces, like `rescanPeople`. */
export const withoutPeople = (project: Project): Project => ({
  ...project,
  masks: project.masks?.filter(({ name }) => name !== 'skin' && !name.startsWith('person:')),
  people: undefined,
  faceBoxes: undefined,
});

/** Reads the saved masks back as they were, so the edits replay with the same masks without running the models. */
export async function loadSavedMasks(project: Project, photo: AbraImage, isCancelled: () => boolean) {
  const readMask = async (file: string, sameSize: boolean) => {
    const image = ownedImage(await readImageAsync(toLocalPath(`${project.dir}/${file}`)));
    const mask = Mask.fromImage(image);
    image.uniffiDestroy();
    if (sameSize && (mask.width() !== photo.width() || mask.height() !== photo.height())) {
      throw new Error(`The saved mask ${file} does not match the project image`);
    }
    return mask;
  };
  const saved = project.masks ?? [];
  const read = await Promise.all(saved.map(async ({ name, file }) => [name, await readMask(file, name === 'skin' || name.startsWith('person:'))] as const));
  if (isCancelled()) return read.forEach(([, mask]) => destroy(mask));
  const byName = Object.fromEntries(read);
  if (byName.skin) skinMask.next(byName.skin);
  // A project saved without a person's mask has no people to give back: they are found again, rather than kept without a mask.
  const loaded = project.people?.map(person => ({ ...person, mask: byName[`person:${person.id}`] }));
  if (loaded?.every(person => person.mask)) people.next(loaded);
  for (const [group, preload] of Object.entries(GROUPS)) {
    // A group that ran and found nothing is saved too, as an empty set.
    if (!project.maskGroups?.includes(group)) continue;
    const prefix = `${group}:`;
    provideAnalysis(
      preload,
      Object.fromEntries(read.filter(([name]) => name.startsWith(prefix)).map(([name, mask]) => [name.slice(prefix.length), mask])),
    );
  }
  // Saved with the people, in the same pixels. An empty object is an answer too: the models ran and found no face.
  if (project.faceBoxes) faceBoxes.next(project.faceBoxes);
}

type SavedMasks = Pick<Project, 'masks' | 'maskGroups' | 'people'>;

/**
 * Saves the masks the project does not have yet, one file at a time, and gives back the project's new `masks`, `maskGroups` and
 * `people`. It pauses (`yield`) after each file, so a caller that must not block can let the screen draw between files. A mask belongs
 * to the original, so it is saved once: the first time it exists.
 */
function* saveMasksSteps(project: Project): Generator<void, SavedMasks> {
  const masks = [...(project.masks ?? [])];
  const groups = [...(project.maskGroups ?? [])];
  const directory = toLocalPath(project.dir);
  const todo: [string, MaskLike][] = [];
  const add = (name: string, mask: MaskLike) => {
    if (!masks.some(saved => saved.name === name) && !todo.some(([queued]) => queued === name)) todo.push([name, mask]);
  };
  if (skinMask.value) add('skin', skinMask.value);
  let savedPeople = project.people;
  // A project saved before the masks were one list has people but not their masks, so those are saved with the people found again.
  const hasMasks = savedPeople?.every(({ id }) => masks.some(saved => saved.name === `person:${id}`));
  if ((!savedPeople || !hasMasks) && people.value) {
    savedPeople = people.value.map(({ mask, ...person }: PersonDetection) => {
      add(`person:${person.id}`, mask);
      return person;
    });
  }
  for (const [group, preload] of Object.entries(GROUPS)) {
    const made = analysisOf(preload);
    if (groups.includes(group) || !made || Array.isArray(made)) continue;
    for (const [name, mask] of Object.entries(made)) add(`${group}:${name}`, mask as MaskLike);
    groups.push(group);
  }
  for (const [name, mask] of todo) {
    const file = `mask-${masks.length}-${Date.now()}.webp`;
    writeMask(mask, `${directory}/${file}`);
    masks.push({ name, file });
    yield;
  }
  return {
    masks: masks.length === (project.masks?.length ?? 0) ? project.masks : masks,
    maskGroups: groups.length === (project.maskGroups?.length ?? 0) ? project.maskGroups : groups,
    people: savedPeople,
  };
}

/** Saves the masks the project does not have yet all at once, for the moment the editor closes. */
export function saveMasks(project: Project): SavedMasks {
  const steps = saveMasksSteps(project);
  let step = steps.next();
  while (!step.done) step = steps.next();
  return step.value;
}

let savingInBackground = false;
let saveAgain = false;

/**
 * Saves the masks the project does not have yet to disk and into `project.json`, without blocking: one file at a time, with the screen
 * free to draw between them. It is run when something new is found, so closing the editor has little left to write and a crash loses
 * nothing that was found. It stops when the project closes or changes under it, because what it was about to write may be freed.
 */
export async function saveMasksInBackground() {
  if (savingInBackground) {
    saveAgain = true;
    return;
  }
  savingInBackground = true;
  try {
    do {
      saveAgain = false;
      const project = openProject.value;
      if (!project || !historyReady.value) return;
      const steps = saveMasksSteps(project);
      let step = steps.next();
      while (!step.done) {
        await new Promise<void>(resolve => setTimeout(resolve, 50));
        // The editor was closed, or the project saved or reset: start again from the new state, if there still is a project.
        if (openProject.value !== project) break;
        step = steps.next();
      }
      if (!step.done || openProject.value !== project) {
        saveAgain = !!openProject.value;
        continue;
      }
      const saved = step.value;
      const boxes = project.faceBoxes ?? faceBoxes.value ?? undefined;
      if (
        saved.masks === project.masks &&
        saved.maskGroups === project.maskGroups &&
        saved.people === project.people &&
        boxes === project.faceBoxes
      ) {
        continue;
      }
      const next = { ...project, ...saved, faceBoxes: boxes };
      openProject.next(next);
      if (__DEV__) console.debug('[masks] saving the project record:', next.masks?.length ?? 0, 'masks, groups', next.maskGroups, 'people', next.people?.length);
      updateProject(next).then(
        () => __DEV__ && console.debug('[masks] project record saved'),
        error => console.warn('[masks] could not save the project:', errorMessage(error)),
      );
    } while (saveAgain);
  } finally {
    savingInBackground = false;
  }
}

/**
 * Builds, in the background, every mask the photo does not have yet and whose models are on this device, so the controls find them
 * ready. A photo with no people gets the skin and the people only. It builds them one after another, never together: each loads large models and the phone runs out of memory when several are
 * loaded at once. It waits for nothing, shows no error, and skips what a project already saved. Anything whose model is missing is left
 * for the moment a control needs it, which offers the download.
 */
export async function buildMasks(photo: AbraImage, isCancelled: () => boolean) {
  const step = async (work: () => Promise<unknown>) => {
    if (isCancelled()) return;
    try {
      await work();
    } catch (error) {
      if (__DEV__) console.debug('[masks] background build failed:', errorMessage(error));
    } finally {
      // The next step loads its own models, so this step's are freed first, and only one step's models are in memory at a time.
      unloadModelsWhenIdle();
      // What this step found goes to disk before the next step starts, because a later step can run the phone out of memory and
      // close the app, and what was only in memory would be found again.
      saveMasksInBackground();
    }
  };
  if (!skinMask.value) await step(() => detectSkin(photo));
  if (!people.value && personDetectionDownloaded()) {
    await step(async () => {
      const found = await detectPeople(photo, { isCancelled });
      if (found.length > 0 && !isCancelled()) await findFaceBoxes(photo);
    });
  }
  // The faces, the hair and the clothes are parts of people, so a photo with none has nothing for them to find. A control that is
  // used on such a photo makes them itself.
  if (!people.value?.length) return;
  for (const preload of Object.values(GROUPS)) await step(() => buildAnalysis(preload, photo));
}
