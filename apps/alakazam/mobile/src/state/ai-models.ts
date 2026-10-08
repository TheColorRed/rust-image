import { BehaviorSubject } from 'rxjs';

/** Bumped after AI models are downloaded, so anything waiting on them can try again. */
export const aiModelsVersion = new BehaviorSubject(0);
export const aiModelsVersion$ = aiModelsVersion.asObservable();

/** Set when the download offer was declined, so it is not shown again until the app restarts. */
export const aiModelsOfferDeclined = new BehaviorSubject(false);
