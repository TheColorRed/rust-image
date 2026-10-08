import { batch } from '@/src/lib/batch';
import { resolveLocalPhotoPath, toLocalPath, type EditablePhoto } from '@/src/lib/edit-source';
import {
  PERSON_MODEL_MISSING,
  clearTimeline,
  editsAPerson,
  healBlemish,
  keepBlemishBase,
} from '@/src/lib/editor-history';
import { startEditorLive } from '@/src/lib/editor-live';
import { clearControlThumbnails, renderRecipe, startReplay } from '@/src/lib/editor-replay';
import { errorMessage } from '@/src/lib/error-message';
import { ownedImage } from '@/src/lib/native-image';
import { projectImageUri, type ProjectRecipe } from '@/src/lib/project-store';
import { recipeAt, type TimelineEntry } from '@/src/lib/timeline';
import { activeSectionKey, adjustments, appliedActions, focusedControlKey } from '@/src/state/edits';
import { timeline } from '@/src/state/history';
import { personSelection } from '@/src/state/person-selection';
import { previewSourceImage, showingCheckpoint } from '@/src/state/preview';
import { historyReady, openProject } from '@/src/state/project-session';
import { updateProject } from '@/src/state/projects';
import { busy, currentImage, editBaseImage, loadError, originalImage, ready } from '@/src/state/session';
import { AbraImage, personDetectionDownloaded } from '@alakazam/mobile';
import { Alert, AppState } from 'react-native';
import { type BehaviorSubject } from 'rxjs';

/** Nulls the subject before destroying its Rust object so no subscriber can read a dangling pointer. */
function destroy<T extends { uniffiDestroy(): void }>(subject: BehaviorSubject<T | null>) {
  const value = subject.value;
  subject.next(null);
  value?.uniffiDestroy();
}

function resetSessionState() {
  destroy(originalImage);
  destroy(currentImage);
  destroy(editBaseImage);
  clearTimeline();
  previewSourceImage.next(null);
  openProject.next(null);
  historyReady.next(false);
  clearControlThumbnails();
  adjustments.next({});
  appliedActions.next([]);
  focusedControlKey.next(null);
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

const nextFrame = () => new Promise<void>(resolve => setTimeout(resolve, 0));

const NOT_SAVED = 'This session will not be saved, so the project keeps what it had.';

/**
 * Saves the open project: every step in the order it was performed, undone ones included, where the cursor is, and the
 * final image at the cursor, so the grid and the next open can show it before anything is replayed. Does nothing for a
 * photo that is not a project, while the project is still loading (its saved state must not be overwritten by an empty
 * one), or when nothing changed since it was opened.
 */
function saveOpenProject() {
  const project = openProject.value;
  const base = editBaseImage.value;
  // Until the history is built the base is only the saved picture, and the steps are not there yet.
  if (!project || !base || !historyReady.value) return;
  const { entries, cursor } = timeline.value;
  // The skin mask belongs to the original, so it is saved once: the first time the image has one.
  const skinMaskFile =
    project.skinMaskFile ??
    (originalImage.value?.writeSkinMask(toLocalPath(`${project.dir}/skin-mask.webp`)) ? 'skin-mask.webp' : undefined);
  if (
    skinMaskFile === project.skinMaskFile &&
    JSON.stringify([entries, cursor]) === JSON.stringify([project.timeline, project.cursor])
  )
    return;

  try {
    // The picture is the base with the edits since the last blemish on top, which is what the controls show now.
    const recipe: ProjectRecipe = { adjustments: { ...adjustments.value }, appliedActions: [...appliedActions.value] };
    const hasRecipe = Object.keys(recipe.adjustments).length > 0 || recipe.appliedActions.length > 0;
    let resultFile: string | undefined;
    if (cursor > 0) {
      const stamp = Date.now();
      resultFile = `result-${stamp}.webp`;
      const final = hasRecipe ? renderRecipe(base, recipe) : base;
      try {
        try {
          final.writeWithQuality(toLocalPath(`${project.dir}/${resultFile}`), RESULT_QUALITY);
        } catch (error) {
          // WebP cannot hold a side over 16383 pixels, and may run out of memory on a very large photo. JPEG still can.
          console.warn('[project] WebP result failed, saving a JPEG instead:', errorMessage(error));
          resultFile = `result-${stamp}.jpg`;
          final.writeWithQuality(toLocalPath(`${project.dir}/${resultFile}`), JPEG_FALLBACK_QUALITY);
        }
      } finally {
        if (final !== base) final.uniffiDestroy();
      }
    }
    const savedProject = { ...project, timeline: entries, cursor, resultFile, skinMaskFile };
    openProject.next(savedProject);
    updateProject(savedProject).catch(error => Alert.alert('Could not save the project', errorMessage(error)));
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
export function openEditorSession(photo: EditablePhoto): () => void {
  let cancelled = false;
  const project = photo.project ?? null;
  const replay = startReplay();
  const live = startEditorLive();
  // Phones kill apps in the background without warning, so save when the app leaves the foreground too.
  const appState = AppState.addEventListener('change', state => {
    if (state !== 'active') saveOpenProject();
  });

  (async () => {
    try {
      if (project?.resultFile) {
        const result = ownedImage(await AbraImage.readAsync(toLocalPath(projectImageUri(project))));
        if (cancelled) {
          result.uniffiDestroy();
          return;
        }
        // The saved picture is shown, and edits can be made on it right away, while the history rebuilds below. Those
        // edits are pending work: they are merged into the saved edits once the real base is ready. The first replay
        // destroys the shown image, like any image it replaces.
        batch(() => {
          editBaseImage.next(result.copy() as AbraImage);
          currentImage.next(result);
          previewSourceImage.next(result);
          ready.next(true);
        });
      }

      const path = await resolveLocalPhotoPath(photo);
      if (cancelled) return;
      const started = Date.now();
      // A saved skin mask is loaded as it was, rather than running the skin model again.
      const original = ownedImage(
        project?.skinMaskFile
          ? await AbraImage.readWithSkinMaskAsync(path, toLocalPath(`${project.dir}/${project.skinMaskFile}`))
          : await AbraImage.readAsync(path),
      );
      if (cancelled) {
        original.uniffiDestroy();
        return;
      }
      if (__DEV__) console.debug('[image] decode and skin mask ms:', Date.now() - started);

      // The project was closed with some steps applied and maybe more undone after them. Only the applied steps are rebuilt.
      // Only blemish steps change pixels, so only they are rendered: each heals the edits that came before it, on top of the
      // picture the last blemish left. Every other step is just edit state, which the last step already holds. Each healed
      // picture is kept by the timeline, so undo and redo across a blemish only swap the base. The undone steps are kept as
      // they are, and a blemish among them is healed if it is ever redone.
      const saved = project?.timeline ?? [];
      const applied = Math.min(project?.cursor ?? saved.length, saved.length);
      let base = original.copy() as AbraImage;
      const rebuilt: TimelineEntry[] = [];
      let restoreFailed = false;
      // Without the person model, an edit made to one person cannot be replayed (it would only throw).
      const personModelMissing = !personDetectionDownloaded();
      for (const entry of saved.slice(0, applied)) {
        try {
          if (personModelMissing && editsAPerson(entry.before ?? entry)) throw new Error(PERSON_MODEL_MISSING);
          if (entry.change.type === 'blemish') {
            await nextFrame();
            if (cancelled) {
              base.uniffiDestroy();
              original.uniffiDestroy();
              return;
            }
            const healed = healBlemish(base, entry);
            keepBlemishBase(entry, healed.copy() as AbraImage);
            base.uniffiDestroy();
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
        editBaseImage.value?.uniffiDestroy();
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
    } catch (e: any) {
      busy.next(false);
      if (!cancelled) loadError.next(String(e?.message ?? e));
    }
  })();

  return () => {
    cancelled = true;
    appState.remove();
    saveOpenProject();
    replay.unsubscribe();
    live.unsubscribe();
    batch(resetSessionState);
  };
}
