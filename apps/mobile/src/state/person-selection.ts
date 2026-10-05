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
