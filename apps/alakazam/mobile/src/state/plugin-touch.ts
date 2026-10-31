import { BehaviorSubject, Subject } from 'rxjs';

/** One finger on the preview for a plugin: `x` and `y` are in the preview's own pixels, from its top-left. */
export type PluginTouch = { type: 'down' | 'move' | 'up' | 'cancel'; x: number; y: number };
export const pluginTouch = new Subject<PluginTouch>();
export const pluginTouch$ = pluginTouch.asObservable();
/** How many plugin listeners want one-finger touches. While there is one, the preview hands a single finger to the plugin instead of panning. */
export const pluginTouchListeners = new BehaviorSubject(0);
export const pluginTouchListeners$ = pluginTouchListeners.asObservable();
