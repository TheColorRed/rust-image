import { BehaviorSubject } from 'rxjs';

/** What the tool last had to say, such as that the removal failed. It clears itself, because a dialog would take the screen's focus in the middle of a touch. */
export const removalMessage = new BehaviorSubject<string | null>(null);
export const removalMessage$ = removalMessage.asObservable();

/** The role of the remover the person picked, or null to use the first the catalog offers. It is not kept when the app closes. */
export const removerRole = new BehaviorSubject<string | null>(null);
export const removerRole$ = removerRole.asObservable();
