import type { Gizmos } from '@alakazam/mobile';
import { BehaviorSubject } from 'rxjs';

/**
 * The gizmos drawn over the editor's preview: lines, outlines and shapes that the library draws on a transparent view above the
 * photo. The preview makes it when it appears and lets go of it when it goes, so a control draws on it only while it is not `null`,
 * and takes what it drew away when it is done. Sizes and positions are in the view's pixels, not dp (multiply by `PixelRatio`).
 */
export const editorGizmos = new BehaviorSubject<Gizmos | null>(null);
export const editorGizmos$ = editorGizmos.asObservable();
