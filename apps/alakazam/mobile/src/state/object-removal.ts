import { BehaviorSubject } from 'rxjs';

/** Short feedback from a completed removal action. */
export const removalMessage = new BehaviorSubject<string | null>(null);
export const removalMessage$ = removalMessage.asObservable();

/** A model load error remains visible until retry, model change, or tool close. */
export const removalLoadError = new BehaviorSubject<string | null>(null);
export const removalLoadError$ = removalLoadError.asObservable();

/** The role of the remover the person picked, or null to use the first the catalog offers. It is not kept when the app closes. */
export const removerRole = new BehaviorSubject<string | null>(null);
export const removerRole$ = removerRole.asObservable();
