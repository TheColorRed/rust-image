import { devDebug, devLog, warn } from '@/src/lib/log';
import { registerAiAnalysis } from '@/src/lib/ai/ai-analyses';
import { clearAnalysis } from '@/src/lib/ai/analysis';
import { SKIN_EVEN_OUT_KEY, SKIN_SMOOTH_KEY, SKIN_TAN_KEY, SKIN_TONE_KEY } from '@/src/lib/project/edit-sections/keys';
import {
  EDGE_REFINEMENT,
  HAIR_MODELS,
  findFaceBoxes,
  MAKEUP_MODELS,
  PERSON_DETECTION,
  SKIN_SEGMENTATION,
} from '@/src/lib/ai/faces';
import { batch } from '@/src/lib/utils/batch';
import { resolveLocalPhotoPath, toLocalPath, type EditablePhoto } from '@/src/lib/editor/edit-source';
import { clearTimeline } from '@/src/lib/editor/editor-history';
import { forgetAnalysis } from '@/src/lib/editor/editor-preloads';
import {
  buildMasks,
  loadSavedMasks,
  saveMasks,
  saveMasksInBackground,
  forgetAnalysisGroup,
  withoutGroup,
  withoutPeople,
} from '@/src/lib/ai/saved-masks';
import { subjectMask } from '@/src/lib/ai/subject';
import { makeupMasks } from '@/src/lib/beauty/makeup-masks';
import { startEditorPreview } from '@/src/lib/editor/editor-preview';
import {
  clearControlThumbnails,
  flushFullReplay,
  markCurrentImageRecipe,
  releaseShownImages,
  renderRecipe,
  restoreProjectRenderCache,
  stackSignatures,
  startReplay,
  writeProjectRenderLayers,
} from '@/src/lib/editor/editor-replay';
import { appliedCrops, fitted, setPersistedCropCount, showStandIn, shrunkOf } from '@/src/lib/editor/editor-pictures';
import { errorMessage } from '@/src/lib/utils/error-message';
import { ownedImage } from '@/src/lib/native/native-image';
import { cacheFileUri, shareImageFile, writeImageToGallery } from '@/src/lib/native/media-file';
import { exportFormat, exportQuality, type ExportFormat } from '@/src/state/settings';
import { projectImageUri, type Project, type ProjectRecipe } from '@/src/lib/project/project-store';
import { changesPixels, emptyTimeline } from '@/src/lib/project/timeline';
import {
  analysisVersion,
  analysisVersion$,
  faceBoxes,
  faceBoxes$,
  people,
  people$,
  skinMask,
  skinMask$,
} from '@/src/state/analysis';
import { aiModelsVersion } from '@/src/state/ai-models';
import { closeRequested } from '@/src/state/commands';
import { activeSectionKey, adjustments, appliedActions, focusedControlKey, openGroupKey } from '@/src/state/edits';
import { orderToolbarSections, toolbarUsage } from '@/src/state/toolbar-usage';
import { clearScrollOffsets } from '@/src/lib/editor/scroll-offsets';
import { EDIT_SECTIONS } from '@/src/lib/project/edit-sections';
import { timeline } from '@/src/state/history';
import { personSelection } from '@/src/state/person-selection';
import { previewSourceImage, showingCheckpoint } from '@/src/state/preview';
import { historyReady, openProject } from '@/src/state/project-session';
import { updateProject } from '@/src/state/projects';
import {
  busy,
  currentImage,
  editBaseImage,
  loadError,
  originalImage,
  ready,
  sessionBaseImage,
} from '@/src/state/session';
import { type Image as AbraPixels } from '@abra/core/image';
import { AbraImage, readImageAsync } from '@alakazam/mobile';
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
  lastSavedBase = null;
  setPersistedCropCount(0);
  destroy(originalImage);
  destroy(currentImage);
  destroy(editBaseImage);
  destroy(sessionBaseImage);
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
 * it keeps transparency. It is used for the project grid and as an immediate stand-in while saved control snapshots load.
 */
const RESULT_QUALITY = 85;
/** The JPEG quality of the saved final image when WebP cannot encode the photo. */
const JPEG_FALLBACK_QUALITY = 90;

// Keep the source project available for reset and delete actions while its editor session is open.
let sessionProject: Project | null = null;
let lastSavedBase: AbraImage | null = null;
/** Horizontal position of the section tabs, retained only while the current editor project session is open. */
export let editorSectionScrollOffset = 0;

export function setEditorSectionScrollOffset(offset: number) {
  editorSectionScrollOffset = offset;
}

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
  forgetAnalysisGroup('hair');
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
  roles: HAIR_MODELS,
  required: HAIR_MODELS,
  usedBy: control => control.analysisGroup === 'hair',
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

/**
 * Saves the document's editable control values and rendered pixel stack. Undo and redo stay in memory for this session only.
 * Does nothing for a photo that is not a project, while it is still loading, or when nothing changed since it was opened.
 */
export async function saveOpenProject(): Promise<void> {
  const project = openProject.value;
  const base = editBaseImage.value;
  if (!project || !base || !historyReady.value) return;
  const { entries, cursor } = timeline.value;
  const activeEntries = entries.slice(0, cursor);
  const recipe: ProjectRecipe = { adjustments: { ...adjustments.value }, appliedActions: [...appliedActions.value] };
  const hasRecipe = Object.keys(recipe.adjustments).length > 0 || recipe.appliedActions.length > 0;
  // The masks belong to the original, so each is saved once: the first time it exists. The project then opens without the models.
  const { masks, maskGroups, people: savedPeople } = saveMasks(project);
  // Like the people, the faces belong to the original, so they are saved once: the first time they are known.
  const savedFaceBoxes = project.faceBoxes ?? faceBoxes.value ?? undefined;
  const savedCrops = appliedCrops();
  const pixelChanges = activeEntries.some(entry => changesPixels(entry.change));
  try {
    if (
      masks === project.masks &&
      maskGroups === project.maskGroups &&
      JSON.stringify(recipe) === JSON.stringify(project.recipe) &&
      (project.layers?.length ?? -1) === stackSignatures(recipe).length &&
      !!project.resultFile === (hasRecipe || !!project.baseFile || cursor > 0) &&
      (!pixelChanges || (base === lastSavedBase && !!project.baseFile)) &&
      JSON.stringify([savedCrops, savedPeople, savedFaceBoxes]) ===
        JSON.stringify([project.savedCrops, project.people, project.faceBoxes])
    )
      return;

    const stamp = Date.now();
    let baseFile = project.baseFile;
    if (pixelChanges) {
      baseFile = `base-${stamp}.png`;
      base.write(toLocalPath(`${project.dir}/${baseFile}`), undefined);
    }
    let layers = project.layers;
    let rendered: AbraImage | null = null;
    if (hasRecipe) {
      // Save each control's full-resolution composite. Reopening can seed the render cache from these images instead of
      // executing every saved control again. The project stores pixels and values, never undo/redo history as a recipe log.
      const stack = writeProjectRenderLayers(base, recipe, (index, _signature, image) => {
        const file = `layer-${stamp}-${index}.png`;
        image.write(toLocalPath(`${project.dir}/${file}`), undefined);
        return file;
      });
      layers = stack.layers;
      rendered = stack.image;
      release(currentImage.value);
      currentImage.next(rendered.clone() as AbraImage);
    } else {
      layers = [];
    }
    let resultFile: string | undefined;
    if (hasRecipe || baseFile || cursor > 0) {
      resultFile = `result-${stamp}.webp`;
      const final = rendered ?? base;
      try {
        try {
          final.write(toLocalPath(`${project.dir}/${resultFile}`), { quality: RESULT_QUALITY });
        } catch (error) {
          // WebP cannot hold a side over 16383 pixels, and may run out of memory on a very large photo. JPEG still can.
          warn('project', 'WebP result failed, saving a JPEG instead:', errorMessage(error));
          resultFile = `result-${stamp}.jpg`;
          final.write(toLocalPath(`${project.dir}/${resultFile}`), { quality: JPEG_FALLBACK_QUALITY });
        }
      } finally {
        release(rendered);
        rendered = null;
      }
      if (!hasRecipe) {
        release(currentImage.value);
        currentImage.next(base.clone() as AbraImage);
      }
      markCurrentImageRecipe(recipe);
    }
    release(rendered);
    const savedProject = {
      ...project,
      recipe,
      layers,
      baseFile,
      savedCrops,
      resultFile,
      masks,
      maskGroups,
      people: savedPeople,
      faceBoxes: savedFaceBoxes,
    };
    devDebug('project', 'saving on close:', savedProject.masks?.length ?? 0, 'masks, groups', savedProject.maskGroups);
    openProject.next(savedProject);
    await updateProject(savedProject);
    lastSavedBase = base;
  } catch (error) {
    Alert.alert('Could not save the project', errorMessage(error));
  }
}

/**
 * Loads `photo` into the editor and restores saved control snapshots into the render cache without rerunning saved controls.
 * Undo and redo start empty for each session. Returns the teardown, which saves the project and destroys session images.
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
  const { id, name, sourceFile, createdAt, dir } = project;
  const fresh: Project = {
    version: 2,
    id,
    name,
    sourceFile,
    recipe: { adjustments: {}, appliedActions: [] },
    layers: [],
    savedCrops: [],
    createdAt,
    modifiedAt: Date.now(),
    dir,
  };
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
  clearScrollOffsets();
  // A newly opened project starts at the first section; closing this editor session clears the offset below.
  editorSectionScrollOffset = 0;
  sessionProject = project;
  setPersistedCropCount(project?.savedCrops.length ?? 0);
  if (project) devLog('project', 'opened', project.id, project.dir);
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
  (async () => {
    try {
      if (project && (project.version !== 2 || !project.recipe || !project.layers)) {
        throw new Error('This project format is no longer supported. Create a new project from the original image.');
      }
      if (
        project &&
        (Object.keys(project.recipe.adjustments).length > 0 || project.recipe.appliedActions.length > 0) &&
        !project.resultFile
      ) {
        throw new Error('This project is missing its saved image.');
      }
      if (project?.resultFile) {
        const result = ownedImage(await readImageAsync(toLocalPath(projectImageUri(project))));
        if (cancelled) {
          release(result);
          return;
        }
        // Show the saved composite immediately. The editor stays unready until the saved control layers are loaded into
        // the render cache, so opening does not execute the saved tools again.
        batch(() => {
          currentImage.next(result);
          showStandIn(result);
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
      devDebug('image', 'decode and skin mask ms:', Date.now() - started);

      let base: AbraImage;
      if (project?.baseFile) {
        base = ownedImage(await readImageAsync(toLocalPath(`${project.dir}/${project.baseFile}`)));
      } else {
        base = original.clone() as AbraImage;
      }
      if (cancelled) {
        release(base);
        release(original);
        return;
      }

      const persistedRecipe: ProjectRecipe = project ? project.recipe : { adjustments: {}, appliedActions: [] };
      const merged = persistedRecipe;
      const savedLayerImages: { signature: string; image: AbraImage }[] = [];
      if (project) {
        const screenBase = shrunkOf('base', base);
        try {
          for (const layer of project.layers) {
            const fullLayer = ownedImage(await readImageAsync(toLocalPath(`${project.dir}/${layer.file}`)));
            if (cancelled) {
              release(fullLayer);
              savedLayerImages.forEach(saved => release(saved.image));
              release(base);
              release(original);
              return;
            }
            try {
              savedLayerImages.push({
                signature: layer.signature,
                image: fitted(fullLayer, screenBase.width(), screenBase.height()),
              });
            } finally {
              release(fullLayer);
            }
          }
        } catch (error) {
          savedLayerImages.forEach(saved => release(saved.image));
          throw error;
        }
      }
      restoreProjectRenderCache(base, merged, savedLayerImages);
      markCurrentImageRecipe(merged);

      const activeProject = project;
      const initialSection = orderToolbarSections(EDIT_SECTIONS, toolbarUsage.value)[0];

      batch(() => {
        release(editBaseImage.value);
        originalImage.next(original);
        editBaseImage.next(base);
        release(sessionBaseImage.value);
        sessionBaseImage.next(base.clone() as AbraImage);
        lastSavedBase = base;
        timeline.next(emptyTimeline);
        if (activeProject) {
          openProject.next(activeProject);
          setPersistedCropCount(activeProject.savedCrops.length);
        }
        adjustments.next(merged.adjustments);
        appliedActions.next(merged.appliedActions);
        if (initialSection) activeSectionKey.next(initialSection.key);
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
    saveOpenProject();
    replay.unsubscribe();
    live.unsubscribe();
    batch(resetSessionState);
    clearScrollOffsets();
    editorSectionScrollOffset = 0;
    if (sessionProject === project) sessionProject = null;
    // Some handles are let go a moment after the close (see `replaceThumbnails`), so what is still alive after that was never released.
    // Only the handles this session made count: a photo opened in the meantime has images of its own.
    if (__DEV__) {
      const closedAt = handleLedgerMark();
      setTimeout(() => warnUnreleasedSince(ledgerMark, 'the editor closed', closedAt), 3000);
    }
  };
}
