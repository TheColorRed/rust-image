import { AbraImage, AbraImageHistory } from '@alakazam/mobile';
import { type BehaviorSubject } from 'rxjs';
import { batch } from '@/src/lib/batch';
import { resolveLocalPhotoPath, type EditablePhoto } from '@/src/lib/edit-source';
import { syncHistoryFlags } from '@/src/lib/editor-history';
import { startEditorLive } from '@/src/lib/editor-live';
import { startReplay } from '@/src/lib/editor-replay';
import { activeSectionKey, adjustments, appliedActions, focusedControlKey } from '@/src/state/edits';
import { draftUiHistory, lastDraftGroup, recordDraft } from '@/src/state/history';
import { controlThumbnails, previewSourceImage, showingCheckpoint } from '@/src/state/preview';
import {
  busy,
  checkpointHistory,
  currentImage,
  draftHistory,
  editBaseImage,
  loadError,
  originalImage,
  ready,
  saved,
} from '@/src/state/session';

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
  destroy(checkpointHistory);
  destroy(draftHistory);
  previewSourceImage.next(null);
  controlThumbnails.next({});
  adjustments.next({});
  appliedActions.next([]);
  focusedControlKey.next(null);
  activeSectionKey.next('section-colorization');
  draftUiHistory.next({ entries: [{ adjustments: {}, appliedActions: [] }], index: 0 });
  recordDraft.next('none');
  lastDraftGroup.next(null);
  showingCheckpoint.next(false);
  saved.next(false);
  busy.next(false);
  loadError.next(null);
  ready.next(false);
  syncHistoryFlags();
}

/**
 * Loads `photo` into the editor controller and starts replaying edits onto it. Returns the
 * teardown, which stops the replay and destroys every native image. Destroying nulls the subjects
 * rather than just freeing the objects: a Fast Refresh remount runs this cleanup on the old
 * instance, and a stale in-flight callback could otherwise read a freed pointer ("Raw pointer
 * value was null").
 */
export function openEditorSession(photo: EditablePhoto): () => void {
  let cancelled = false;
  const replay = startReplay();
  const live = startEditorLive();

  (async () => {
    try {
      const path = await resolveLocalPhotoPath(photo);
      if (cancelled) return;
      const original = AbraImage.read(path) as AbraImage;
      batch(() => {
        originalImage.next(original);
        editBaseImage.next(original.copy() as AbraImage);
        checkpointHistory.next(new AbraImageHistory(original));
        draftHistory.next(new AbraImageHistory(original));
        ready.next(true);
      });
    } catch (e: any) {
      if (!cancelled) loadError.next(String(e?.message ?? e));
    }
  })();

  return () => {
    cancelled = true;
    replay.unsubscribe();
    live.unsubscribe();
    batch(resetSessionState);
  };
}
