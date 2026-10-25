import { BehaviorSubject } from 'rxjs';

/** An optional AI model choice, by tool control key. Tools with no choice use their own default. */
export const toolAiModelSelections = new BehaviorSubject<Record<string, string>>({});
export const toolAiModelSelections$ = toolAiModelSelections.asObservable();

export function setToolAiModel(key: string, role: string): void {
  toolAiModelSelections.next({ ...toolAiModelSelections.value, [key]: role });
}
