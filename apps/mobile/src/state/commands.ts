import { Subject } from 'rxjs';
import { type EditControl, type SliderControl } from '@/src/lib/edit-sections';

// Fire-and-forget editor commands: UI emits them, whichever component or tool owns the behavior subscribes.

export const closeRequested = new Subject<void>();
export const closeRequested$ = closeRequested.asObservable();
export const selectControlRequested = new Subject<EditControl>();
export const selectControlRequested$ = selectControlRequested.asObservable();
export const selectSectionRequested = new Subject<string>();
export const selectSectionRequested$ = selectSectionRequested.asObservable();
export const sliderCommitRequested = new Subject<{ control: SliderControl; value: number }>();
export const sliderCommitRequested$ = sliderCommitRequested.asObservable();
export const resetRequested = new Subject<void>();
export const resetRequested$ = resetRequested.asObservable();
export const saveRequested = new Subject<void>();
export const saveRequested$ = saveRequested.asObservable();
export const undoRequested = new Subject<void>();
export const undoRequested$ = undoRequested.asObservable();
export const redoRequested = new Subject<void>();
export const redoRequested$ = redoRequested.asObservable();
export const showCheckpointRequested = new Subject<void>();
export const showCheckpointRequested$ = showCheckpointRequested.asObservable();
export const restorePreviewRequested = new Subject<void>();
export const restorePreviewRequested$ = restorePreviewRequested.asObservable();
export const blemishApplyRequested = new Subject<void>();
export const blemishApplyRequested$ = blemishApplyRequested.asObservable();
