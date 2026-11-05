import { BehaviorSubject, Subject } from 'rxjs';

/** One finger on the preview for a plugin: `x` and `y` are in the preview's own pixels, from its top-left. */
export type PluginTouch = { type: 'down' | 'move' | 'up' | 'cancel'; x: number; y: number };
export const pluginTouch = new Subject<PluginTouch>();
export const pluginTouch$ = pluginTouch.asObservable();
/**
 * Two fingers on the preview for a plugin. `x` and `y` are the middle between the fingers in the preview's pixels from its top-left;
 * `scale` is how far apart the fingers are compared with when they went down, `rotation` how far the line between them has turned in degrees
 * (more than a half turn is kept), and `dx` and `dy` how far the middle has moved, in the preview's pixels.
 */
export type PluginPinch = { type: 'start' | 'change' | 'end'; x: number; y: number; scale: number; rotation: number; dx: number; dy: number };
export const pluginPinch = new Subject<PluginPinch>();
export const pluginPinch$ = pluginPinch.asObservable();
/** How many plugin listeners want two-finger gestures. While there is one, the preview hands two fingers to the plugin instead of zooming, turning and moving the camera. */
export const pluginPinchListeners = new BehaviorSubject(0);
/** A plugin's say over which two-finger gestures it takes, given where the middle between the fingers went down in page pixels. With none, a listener takes every pinch. */
export const pluginPinchClaim = new BehaviorSubject<((pageX: number, pageY: number) => boolean) | null>(null);

// Who owns the two fingers on the preview now: decided once, when the second finger lands, so a middle that drifts out of the plugin's place
// does not hand a pinch that began there to the camera halfway through.
let pinchTaken: boolean | null = null;

/** Whether the two fingers `touches` belong to a plugin (true) or to the preview's camera (false). With fewer than two fingers there is no pinch, and the next one is decided afresh. */
export const pluginTakesPinch = (touches: readonly { pageX: number; pageY: number }[]) => {
  if (touches.length < 2) {
    pinchTaken = null;
    return false;
  }
  if (pinchTaken === null) {
    const [first, second] = touches;
    const claim = pluginPinchClaim.value;
    pinchTaken = pluginPinchListeners.value > 0 && (!claim || claim((first.pageX + second.pageX) / 2, (first.pageY + second.pageY) / 2));
  }
  return pinchTaken;
};

/** How many plugin listeners want one-finger touches. While there is one, the preview hands a single finger to the plugin instead of panning. */
export const pluginTouchListeners = new BehaviorSubject(0);
export const pluginTouchListeners$ = pluginTouchListeners.asObservable();
