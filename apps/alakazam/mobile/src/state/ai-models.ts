import { BehaviorSubject } from 'rxjs';

/** Bumped after AI models are downloaded, so anything waiting on them can try again. */
export const aiModelsVersion = new BehaviorSubject(0);
export const aiModelsVersion$ = aiModelsVersion.asObservable();

/** Set when the download offer was declined, so it is not shown again until the app restarts. */
export const aiModelsOfferDeclined = new BehaviorSubject(false);

/**
 * An action that needs AI models that are not on this device was tapped. The prompt asks to download them; `onReady` runs
 * the action again once they are. Cancelling drops the request and nothing is downloaded.
 */
export type AiModelsRequest = { title: string; roles: readonly string[]; onReady: () => void };
export const aiModelsRequested = new BehaviorSubject<AiModelsRequest | null>(null);
export const aiModelsRequested$ = aiModelsRequested.asObservable();
