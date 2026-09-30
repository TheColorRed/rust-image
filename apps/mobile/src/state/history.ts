import { BehaviorSubject } from 'rxjs';

// #region: Undo / redo availability

export const canUndo = new BehaviorSubject(false);
export const canUndo$ = canUndo.asObservable();
export const canRedo = new BehaviorSubject(false);
export const canRedo$ = canRedo.asObservable();
export const canUndoDraft = new BehaviorSubject(false);
export const canUndoDraft$ = canUndoDraft.asObservable();
export const canRedoDraft = new BehaviorSubject(false);
export const canRedoDraft$ = canRedoDraft.asObservable();

// #endregion

// #region: Draft history

/** UI metadata paired with Rust's image-only draft snapshots, one entry per draft history step. */
export interface DraftUiState {
  adjustments: Record<string, number>;
  appliedActions: string[];
}
export const draftUiHistory = new BehaviorSubject<{ entries: DraftUiState[]; index: number }>({
  entries: [{ adjustments: {}, appliedActions: [] }],
  index: 0,
});
export const draftUiHistory$ = draftUiHistory.asObservable();
export const recordDraft = new BehaviorSubject<'none' | 'push' | 'replace'>('none');
export const recordDraft$ = recordDraft.asObservable();
export const lastDraftGroup = new BehaviorSubject<string | null>(null);
export const lastDraftGroup$ = lastDraftGroup.asObservable();

// #endregion
