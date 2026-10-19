import { BehaviorSubject } from 'rxjs';
import { type CropAspect, type ScreenRect } from '@/src/lib/crop';

/** The crop frame drawn over the preview while the Crop tool is open, in dp from the preview's top-left. It stays put on screen while the photo zooms and pans under it. */
export const cropFrame = new BehaviorSubject<ScreenRect | null>(null);
export const cropFrame$ = cropFrame.asObservable();
export const cropAspect = new BehaviorSubject<CropAspect>('freeform');
export const cropAspect$ = cropAspect.asObservable();
/** Whether a finger is dragging a corner handle. While it is, the preview leaves that finger to the handle; any other finger pans the photo. */
export const cropHandleGrabbed = new BehaviorSubject(false);
