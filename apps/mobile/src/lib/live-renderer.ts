import { ImagePreview, Message, type AbraImage } from '@alakazam/mobile';
import { BehaviorSubject } from 'rxjs';

type Open = {
  sourceKey: string;
  /** The size of the photo; only its proportions matter, for laying the view out. */
  width: number;
  height: number;
  preview: ImagePreview;
};

/**
 * A live preview that stays connected to its native view, so showing an effect costs the effect and not a new GPU
 * connection. The preview is a `ImagePreview` that follows an image: this only says which control was touched (`slide`,
 * `act`), and Rust works out the effects and decides when to draw. Make one per place on screen that shows a preview; the
 * screen gives the open `view` to a `VesselView`.
 *
 * `ensure` opens a preview of an image once, and later calls with the same source key reuse it. `hide` takes it off the
 * screen without letting go of the GPU. Frames are drawn on the native view and never pass through JavaScript.
 */
export class LiveRenderer {
  private open: Open | null = null;

  /** The open preview, for a `VesselView` to show; `null` when there is none. */
  readonly view = new BehaviorSubject<ImagePreview | null>(null);

  /**
   * @param surface Receives the size of the photo while its frames are visible, and `null` while hidden. The screen shows
   * the view when it is not `null`.
   */
  constructor(readonly surface: BehaviorSubject<{ width: number; height: number } | null>) {}

  /**
   * Makes sure a preview of the image is open. The same `sourceKey` as before reuses the open preview at once; anything
   * else replaces it. `source` is only called when a new preview is needed, and `ownsSource` says whether to free its
   * image afterwards (the preview keeps its own handle on it). Returns whether a preview is open.
   */
  ensure(sourceKey: string, source: () => AbraImage | null, ownsSource = false): boolean {
    if (this.open?.sourceKey === sourceKey) return true;
    this.release();
    const image = source();
    if (!image) return false;
    try {
      this.open = {
        sourceKey,
        width: image.width(),
        height: image.height(),
        preview: new ImagePreview(image),
      };
      this.view.next(this.open.preview);
      return true;
    } catch (error) {
      console.warn(`[live] could not open a preview:`, error);
      return false;
    } finally {
      if (ownsSource) image.uniffiDestroy();
    }
  }

  /** Puts the frames on the screen without sending anything, for a control that talks to the preview itself. */
  show() {
    const open = this.open;
    if (open) this.publish(open);
  }

  /** Shows the slider `key` at `value` over the image. Send one on every change. */
  slide(key: string, value: number) {
    this.send(Message.SliderMove.new(key, value));
  }

  /** Shows the action `key` over the image. */
  act(key: string) {
    this.send(Message.Action.new(key));
  }

  /** Shows the image as it is. */
  clear() {
    this.send(Message.Action.new(''));
  }

  /**
   * Takes the frames off the screen. The preview stays open and connected for the next use, and keeps what it shows.
   * Clearing it here would redraw the untouched image into the hidden view, and the next time the view is shown it would
   * flash that image for a frame before the new effects reach it.
   */
  hide() {
    this.surface.next(null);
  }

  /** Lets go of the preview and the GPU connection. */
  release() {
    const open = this.open;
    this.open = null;
    this.view.next(null);
    this.surface.next(null);
    open?.preview.uniffiDestroy();
  }

  private publish(open: Open) {
    const current = this.surface.value;
    if (current?.width !== open.width || current.height !== open.height) {
      this.surface.next({ width: open.width, height: open.height });
    }
  }

  private send(message: Message) {
    const open = this.open;
    if (!open) return;
    try {
      open.preview.send(message);
      this.publish(open);
    } catch (error) {
      console.warn(`[live] could not update the preview:`, error);
    }
  }
}
