import { BehaviorSubject } from 'rxjs';
import { type PersonDetection } from '@alakazam/mobile';

export type PersonSelectionState = {
  focused: boolean;
  loading: boolean;
  people: PersonDetection[];
  selectedId: number | null;
  error: string | null;
};

export const personSelection = new BehaviorSubject<PersonSelectionState>({
  focused: false,
  loading: false,
  people: [],
  selectedId: null,
  error: null,
});
export const personSelection$ = personSelection.asObservable();

export const personOutlinesVisible = new BehaviorSubject(false);
export const personOutlinesVisible$ = personOutlinesVisible.asObservable();

/** Whether the user has anything to choose: more than one person was found, or detection is still running or has failed. */
export const selectionNeeded = (state: PersonSelectionState) => state.loading || !!state.error || state.people.length > 1;
