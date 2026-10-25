import { BehaviorSubject } from 'rxjs';

/** The installed object remover tool's own key, registered from its control declaration. */
export const objectRemovalToolKey = new BehaviorSubject<string | null>(null);
export const objectRemovalToolKey$ = objectRemovalToolKey.asObservable();
