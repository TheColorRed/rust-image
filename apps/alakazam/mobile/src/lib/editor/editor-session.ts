import { registerAiAnalysis } from '@/src/lib/ai/ai-analyses';
import { clearAnalysis, detectPeople } from '@/src/lib/ai/analysis';
import { SKIN_EVEN_OUT_KEY, SKIN_SMOOTH_KEY, SKIN_TAN_KEY, SKIN_TONE_KEY } from '@/src/lib/project/edit-sections/keys';
import { EDGE_REFINEMENT, HAIR_MODELS, findFaceBoxes, MAKEUP_MODELS, PERSON_DETECTION, SKIN_SEGMENTATION } from '@/src/lib/ai/faces';
import { batch } from '@/src/lib/utils/batch';
import { resolveLocalPhotoPath, toLocalPath, type EditablePhoto } from '@/src/lib/editor/edit-source';
import {
  PERSON_MODEL_MISSING,
  checkpointFor,
  clearTimeline,
  editsAPerson,
  healBlemish,
  keepBlemishBase,
} from '@/src/lib/editor/editor-history';
import { analysisOf, forgetAnalysis } from '@/src/lib/editor/editor-preloads';
import { buildMasks, loadSavedMasks, saveMasks, saveMasksInBackground, withoutGroup, withoutPeople } from '@/src/lib/ai/saved-masks';
import { subjectMask } from '@/src/lib/ai/subject';
import { makeupMasks } from '@/src/lib/beauty/makeup-masks';
import { hairMasks } from '@/src/lib/beauty/hair-masks';
import { startEditorPreview } from '@/src/lib/editor/editor-preview';
import {
  clearControlThumbnails,
  flushFullReplay,
  releaseShownImages,
  renderRecipe,
  startReplay,
} from '@/src/lib/editor/editor-replay';
import { showStandIn } from '@/src/lib/editor/editor-pictures';
import { errorMessage } from '@/src/lib/utils/error-message';
import { ownedImage } from '@/src/lib/native/native-image';
import { cacheFileUri, shareImageFile, writeImageToGallery } from '@/src/lib/native/media-file';
import { exportFormat, exportQuality, type ExportFormat } from '@/src/state/settings';
import { projectImageUri, type Project, type ProjectRecipe, type SavedPersonDetection } from '@/src/lib/project/project-store';
import { changesPixels, recipeAt, type TimelineEntry } from '@/src/lib/project/timeline';
import { analysisVersion, analysisVersion$, faceBoxes, faceBoxes$, people, people$, skinMask, skinMask$ } from '@/src/state/analysis';
import { aiModelsVersion } from '@/src/state/ai-models';
import { closeRequested } from '@/src/state/commands';
import { activeSectionKey, adjustments, appliedActions, focusedControlKey, openGroupKey } from '@/src/state/edits';
import { timeline, timeline$ } from '@/src/state/history';
import { personSelection } from '@/src/state/person-selection';
import { previewSourceImage, showingCheckpoint } from '@/src/state/preview';
import { historyReady, openProject } from '@/src/state/project-session';
import { updateProject } from '@/src/state/projects';
import { busy, currentImage, editBaseImage, loadError, originalImage, ready } from '@/src/state/session';
import { type Image as AbraPixels } from '@abra/core/image';
import { Mask, type MaskLike } from '@abra/core/mask';
import { AbraImage, personDetectionDownloaded, readImageAsync, type PersonDetection } from '@alakazam/mobile';
import { Alert, AppState } from 'react-native';
import { merge, type BehaviorSubject } from 'rxjs';
import { debounceTime } from 'rxjs/operators';
import { handleLedgerMark, warnUnreleasedSince } from '@/src/lib/native/handle-ledger';
import { release } from '@/src/lib/native/native-handles';

/** Nulls the subject before destroying its Rust object so no subscriber can read a dangling pointer. */
function destroy<T>(subject: BehaviorSubject<T | null>) {
  const value = subject.value;
  subject.next(null);
  release(value);
}

function resetSessionState() {
  destroy(originalImage);
  destroy(currentImage);
  destroy(editBaseImage);
  clearTimeline();
  previewSourceImage.next(null);
  releaseShownImages();
  openProject.next(null);
  historyReady.next(false);
  clearControlThumbnails();
  clearAnalysis();
  adjustments.next({});
  appliedActions.next([]);
  focusedControlKey.next(null);
  openGroupKey.next(null);
  activeSectionKey.next('section-colorization');
  showingCheckpoint.next(false);
  personSelection.next({ focused: false, loading: false, people: [], selectedId: null, error: null });
  busy.next(false);
  loadError.next(null);
  ready.next(false);
}

/**
 * The quality (0 to 100) of a project's saved final image, which is WebP: close to a JPEG at 90 in looks, but smaller, and
 * it keeps transparency. It is only shown before the edits are replayed, so it need not be lossless.
 */
const RESULT_QUALITY = 85;
/** The JPEG quality of the saved final image when WebP cannot encode the photo. */
const JPEG_FALLBACK_QUALITY = 90;

// Keep the source project available for recovery actions even if its saved history cannot be replayed.
let sessionProject: Project | null = null;

const nextFrame = () => new Promise<void>(resolve => setTimeout(resolve, 0));

/**
 * Finds the faces' masks again, in case the first time went wrong: what was found is thrown away, the editor draws again without it,
 * which starts the models, and the new masks are saved over the old ones when the project is saved.
 */
export function rescanFaceMasks() {
  forgetAnalysis(makeupMasks);
  const project = openProject.value;
  if (project) openProject.next(withoutGroup(project, 'makeup'));
  analysisVersion.next(analysisVersion.value + 1);
}

/** Finds the hair again, in case the first time went wrong, like `rescanFaceMasks`. */
export function rescanHair() {
  forgetAnalysis(hairMasks);
  const project = openProject.value;
  if (project) openProject.next(withoutGroup(project, 'hair'));
  analysisVersion.next(analysisVersion.value + 1);
}

/**
 * Finds the skin and the people again, in case the first time went wrong: what was found is thrown away along with what the project
 * saved of it, and the person selector, which runs when a control that uses them is open, finds them again. The selector's own list is
 * emptied first, because it holds the masks that are freed here.
 */
export function rescanPeople() {
  const project = openProject.value;
  if (project) openProject.next(withoutPeople(project));
  personSelection.next({ ...personSelection.value, people: [], selectedId: null, loading: true });
  clearAnalysis();
  analysisVersion.next(analysisVersion.value + 1);
  aiModelsVersion.next(aiModelsVersion.value + 1);
}

/** Finds the cutout of the people again, like `rescanHair`. */
export function rescanSubject() {
  forgetAnalysis(subjectMask);
  const project = openProject.value;
  if (project) openProject.next(withoutGroup(project, 'subject'));
  analysisVersion.next(analysisVersion.value + 1);
}

registerAiAnalysis('makeup', {
  roles: MAKEUP_MODELS,
  required: MAKEUP_MODELS,
  usedBy: control => control.preload === makeupMasks,
  regenerate: rescanFaceMasks,
});
registerAiAnalysis('hair', {
  roles: [...HAIR_MODELS, EDGE_REFINEMENT],
  required: HAIR_MODELS,
  usedBy: control => control.preload === hairMasks,
  regenerate: rescanHair,
});
registerAiAnalysis('subject', {
  roles: [PERSON_DETECTION, SKIN_SEGMENTATION, EDGE_REFINEMENT],
  required: [PERSON_DETECTION],
  usedBy: control => control.preload === subjectMask,
  regenerate: rescanSubject,
});
registerAiAnalysis('people', {
  roles: [SKIN_SEGMENTATION, PERSON_DETECTION],
  required: [SKIN_SEGMENTATION, PERSON_DETECTION],
  usedBy: control => [SKIN_EVEN_OUT_KEY, SKIN_SMOOTH_KEY, SKIN_TAN_KEY, SKIN_TONE_KEY].includes(control.key),
  regenerate: rescanPeople,
});

const NOT_SAVED = 'This session will not be saved, so the project keeps what it had.';

/**
 * Saves the open project: every step in the order it was performed, undone ones included, where the cursor is, and the
 * final image at the cursor, so the grid and the next open can show it before anything is replayed. Does nothing for a
 * photo that is not a project, while the project is still loading (its saved state must not be overwritten by an empty
 * one), or when nothing changed since it was opened.
 */
export async function saveOpenProject(): Promise<void> {
  const project = openProject.value;
  const base = editBaseImage.value;
  // Until the history is built the base is only the saved picture, and the steps are not there yet.
  if (!project || !base || !historyReady.value) return;
  const { entries, cursor } = timeline.value;
  // The masks belong to the original, so each is saved once: the first time it exists. The project then opens without the models.
  const { masks, maskGroups, people: savedPeople } = saveMasks(project);
  // Like the people, the faces belong to the original, so they are saved once: the first time they are known.
  const savedFaceBoxes = project.faceBoxes ?? faceBoxes.value ?? undefined;
  try {
    const entriesWithResources = entries.map((entry, index) => {
      if (!changesPixels(entry.change)) return entry;
      const checkpoint = checkpointFor(entry);
      if (!checkpoint) return entry;
      const checkpointFile = entry.checkpointFile ?? `checkpoint-${index}.webp`;
      checkpoint.write(toLocalPath(`${project.dir}/${checkpointFile}`), undefined);
      return entry.checkpointFile === checkpointFile ? entry : { ...entry, checkpointFile };
    });
    if (
      masks === project.masks &&
      maskGroups === project.maskGroups &&
      JSON.stringify([entriesWithResources, cursor, savedPeople, savedFaceBoxes]) ===
        JSON.stringify([project.timeline, project.cursor, project.people, project.faceBoxes])
    )
      return;

    // The picture is the base with the edits since the last blemish on top, which is what the controls show now.
    const recipe: ProjectRecipe = { adjustments: { ...adjustments.value }, appliedActions: [...appliedActions.value] };
    const hasRecipe = Object.keys(recipe.adjustments).length > 0 || recipe.appliedActions.length > 0;
    let resultFile: string | undefined;
    if (cursor > 0) {
      const stamp = Date.now();
      resultFile = `result-${stamp}.webp`;
      // The replay already made the full-size picture of this recipe (`currentImage`), so it is written as it is. Rendering it again
      // would hold a second full-size copy, and every layer of a canvas, next to the original, the base and that one.
      if (hasRecipe) flushFullReplay();
      const rendered = hasRecipe && !currentImage.value ? renderRecipe(base, recipe) : null;
      const final = rendered ?? (hasRecipe ? currentImage.value! : base);
      try {
        try {
          final.write(toLocalPath(`${project.dir}/${resultFile}`), { quality: RESULT_QUALITY });
        } catch (error) {
          // WebP cannot hold a side over 16383 pixels, and may run out of memory on a very large photo. JPEG still can.
          console.warn('[project] WebP result failed, saving a JPEG instead:', errorMessage(error));
          resultFile = `result-${stamp}.jpg`;
          final.write(toLocalPath(`${project.dir}/${resultFile}`), { quality: JPEG_FALLBACK_QUALITY });
        }
      } finally {
        release(rendered);
      }
    }
    const savedProject = { ...project, timeline: entriesWithResources, cursor, resultFile, masks, maskGroups, people: savedPeople, faceBoxes: savedFaceBoxes };
    if (__DEV__) console.debug('[project] saving on close:', savedProject.masks?.length ?? 0, 'masks, groups', savedProject.maskGroups);
    openProject.next(savedProject);
    await updateProject(savedProject);
  } catch (error) {
    Alert.alert('Could not save the project', errorMessage(error));
  }
}

/**
 * Loads `photo` into the editor controller and starts replaying edits onto it. If `photo` is a project, its saved
 * final image is shown at once while the original is read and the pictures its blemish steps left are rebuilt, in the
 * order they were performed. Returns the teardown, which saves the project, stops the replay and destroys every native
 * image. Destroying nulls the subjects rather than just freeing the objects: a Fast Refresh remount runs this cleanup on
 * the old instance, and a stale in-flight callback could otherwise read a freed pointer ("Raw pointer value was null").
 */
/**
 * For developers: makes the open project as it was when it was first opened. Every edit, the final image, and everything the models
 * found (the masks, the people, the face boxes) are taken off the project, and the editor is closed without saving, so opening the
 * project again starts from the photo alone. The files they were in are left in the project's folder, unused, until it is deleted.
 */
export async function resetProjectToNew() {
  const project = openProject.value ?? sessionProject;
  if (!project) return;
  const { version, id, name, sourceFile, createdAt, dir } = project;
  const fresh: Project = { version, id, name, sourceFile, timeline: [], cursor: 0, createdAt, modifiedAt: Date.now(), dir };
  // Nothing may save over it: not the save when the editor closes, and not the masks being saved in the background.
  historyReady.next(false);
  openProject.next(fresh);
  await updateProject(fresh);
  closeRequested.next();
}

/**
 * Writes the photo as it is edited now, full size, to the cache and opens the phone's share sheet with it. The file is in the
 * export format chosen in Settings, and it stays in the cache until the next share.
 */
export async function shareEditedPhoto() {
  const base = editBaseImage.value;
  if (!base) return;
  const recipe: ProjectRecipe = { adjustments: { ...adjustments.value }, appliedActions: [...appliedActions.value] };
  const hasRecipe = Object.keys(recipe.adjustments).length > 0 || recipe.appliedActions.length > 0;
  if (hasRecipe) flushFullReplay();
  const rendered = hasRecipe && !currentImage.value ? renderRecipe(base, recipe) : null;
  const final = rendered ?? (hasRecipe ? currentImage.value! : base);
  let uri: string | undefined;
  try {
    uri = await cacheFileUri(`${openProject.value?.name || 'photo'}.${exportFormat.value}`);
    final.write(toLocalPath(uri), { quality: exportQuality.value });
  } finally {
    release(rendered);
  }
  // Not deleted here: the app picked in the share sheet reads the file later. The next share clears it.
  await shareImageFile(uri);
}

/** Writes the current full-size editor result to the device photo library in the chosen format. */
export async function saveEditedPhotoToGallery(fileName: string, format: ExportFormat, quality: number): Promise<void> {
  const base = editBaseImage.value;
  if (!base) throw new Error('The photo is still loading. Try again in a moment.');
  const recipe: ProjectRecipe = { adjustments: { ...adjustments.value }, appliedActions: [...appliedActions.value] };
  const hasRecipe = Object.keys(recipe.adjustments).length > 0 || recipe.appliedActions.length > 0;
  if (hasRecipe) flushFullReplay();
  const rendered = hasRecipe && !currentImage.value ? renderRecipe(base, recipe) : null;
  const final = rendered ?? (hasRecipe ? currentImage.value! : base);
  try {
    await writeImageToGallery(final, fileName, format, quality);
  } finally {
    release(rendered);
  }
}

export function openEditorSession(photo: EditablePhoto): () => void {
  let cancelled = false;
  const project = photo.project ?? null;
  sessionProject = project;
  if (__DEV__ && project) console.log('[project] opened', project.id, project.dir);
  const ledgerMark = handleLedgerMark();
  const replay = startReplay();
  const live = startEditorPreview();
  // Phones kill apps in the background without warning, so save when the app leaves the foreground too.
  const appState = AppState.addEventListener('change', state => {
    if (state !== 'active') saveOpenProject();
  });
  // What the models find is saved as soon as it is found, a file at a time without blocking, so closing the editor has little left to
  // write and a crash loses nothing that was found.
  const masksFound = merge(analysisVersion$, people$, skinMask$, faceBoxes$)
    .pipe(debounceTime(300))
    .subscribe(() => saveMasksInBackground());
  // Pixel-changing steps need their checkpoint on disk so reopening never reruns an AI tool.
  const timelineSaved = timeline$.pipe(debounceTime(500)).subscribe(() => saveOpenProject());

  (async () => {
    try {
      if (project?.resultFile) {
        const result = ownedImage(await readImageAsync(toLocalPath(projectImageUri(project))));
        if (cancelled) {
          release(result);
          return;
        }
        // The saved picture is shown, and edits can be made on it right away, while the history rebuilds below. Those
        // edits are pending work: they are merged into the saved edits once the real base is ready. The first replay
        // destroys the shown image, like any image it replaces.
        batch(() => {
          editBaseImage.next(result.clone() as AbraImage);
          currentImage.next(result);
          showStandIn(result);
          ready.next(true);
        });
      }

      const path = await resolveLocalPhotoPath(photo);
      if (cancelled) return;
      const started = Date.now();
      // A saved skin mask is loaded as it was, rather than running the skin model again.
      const original = ownedImage(await readImageAsync(path));
      if (cancelled) {
        release(original);
        return;
      }
      if (project) await loadSavedMasks(project, original, () => cancelled);
      if (cancelled) {
        release(original);
        return;
      }
      if (__DEV__) console.debug('[image] decode and skin mask ms:', Date.now() - started);

      // The project was closed with some steps applied and maybe more undone after them. Only the applied steps are rebuilt.
      // Only steps that change pixels (blemish and red-eye fixes) are rendered: each bakes in the edits that came before it, on top
      // of the picture the last one left. Every other step is just edit state, which the last step already holds. Each healed
      // picture is kept by the timeline, so undo and redo across a blemish only swap the base. The undone steps are kept as
      // they are, and a blemish among them is healed if it is ever redone.
      const saved = project?.timeline ?? [];
      const applied = Math.min(project?.cursor ?? saved.length, saved.length);
      // Load rendered retouch checkpoints up front, including undone ones. Redo can then swap pixels without rerunning the
      // automatic red-eye models (and old projects without checkpoints still use the compatibility replay below).
      for (const entry of saved) {
        if (!changesPixels(entry.change) || !entry.checkpointFile) continue;
        const checkpoint = ownedImage(
          await readImageAsync(toLocalPath(`${project!.dir}/${entry.checkpointFile}`)),
        );
        // The editor may have been left while this was decoding: the session's maps are already cleared, so a picture kept now would
        // stay in memory with no owner.
        if (cancelled) {
          release(checkpoint);
          release(original);
          return;
        }
        keepBlemishBase(entry, checkpoint);
      }
      let base = original.clone() as AbraImage;
      const rebuilt: TimelineEntry[] = [];
      let restoreFailed = false;
      // Without the person model, an edit made to one person cannot be replayed (it would only throw).
      const personModelMissing = !personDetectionDownloaded() && !project?.people;
      for (const entry of saved.slice(0, applied)) {
        try {
          if (personModelMissing && editsAPerson(entry.before ?? entry)) throw new Error(PERSON_MODEL_MISSING);
          if (changesPixels(entry.change)) {
            const checkpoint = checkpointFor(entry);
            let healed: AbraImage;
            if (checkpoint) {
              healed = checkpoint.clone() as AbraImage;
            } else {
              await nextFrame();
              if (cancelled) {
                release(base);
                release(original);
                return;
              }
              healed = healBlemish(base, entry);
              keepBlemishBase(entry, healed.clone() as AbraImage);
            }
            release(base);
            base = healed;
          }
          rebuilt.push(entry);
        } catch (error) {
          // Keep the steps before this one and edit from there, but do not save over the project, which has every step.
          restoreFailed = true;
          Alert.alert('Could not restore every edit', `${errorMessage(error)}\n\n${NOT_SAVED}`);
          break;
        }
      }

      batch(() => {
        // Swap the saved picture, which was only a stand-in base, for the real one.
        release(editBaseImage.value);
        originalImage.next(original);
        editBaseImage.next(base);
        // Edits made while this was loading are pending work. They go on top of the saved edits, and win where both set
        // the same control. They become one step on the timeline.
        const current = recipeAt(rebuilt, rebuilt.length);
        const merged: ProjectRecipe = {
          adjustments: { ...current.adjustments, ...adjustments.value },
          appliedActions: [
            ...current.appliedActions,
            ...appliedActions.value.filter(key => !current.appliedActions.includes(key)),
          ],
        };
        const changed = JSON.stringify(merged) !== JSON.stringify(current);
        // Without a failure the steps after the cursor stay for redo, unless a pending edit comes first and drops them.
        const kept = restoreFailed || changed ? [] : saved.slice(applied);
        const entries: TimelineEntry[] = [
          ...rebuilt,
          ...(changed ? [{ change: { type: 'pending' }, ...merged } as TimelineEntry] : []),
          ...kept,
        ];
        timeline.next({ entries, cursor: rebuilt.length + (changed ? 1 : 0) });
        if (project && !restoreFailed) openProject.next(project);
        // Setting the edit state replays everything from the real base, which also replaces the stand-in picture on screen.
        adjustments.next(merged.adjustments);
        appliedActions.next(merged.appliedActions);
        busy.next(false);
        historyReady.next(true);
        ready.next(true);
      });

      // Every mask the photo is missing is built in the background now that the editor is usable, so the controls find them ready.
      // Nothing waits for this and nothing saved is made again (see `buildMasks`).
      buildMasks(original, () => cancelled);
    } catch (e: any) {
      busy.next(false);
      if (!cancelled) loadError.next(String(e?.message ?? e));
    }
  })();

  return () => {
    cancelled = true;
    appState.remove();
    masksFound.unsubscribe();
    timelineSaved.unsubscribe();
    saveOpenProject();
    replay.unsubscribe();
    live.unsubscribe();
    batch(resetSessionState);
    if (sessionProject === project) sessionProject = null;
    // Some handles are let go a moment after the close (see `replaceThumbnails`), so what is still alive after that was never released.
    if (__DEV__) setTimeout(() => warnUnreleasedSince(ledgerMark, 'the editor closed'), 3000);
  };
}
