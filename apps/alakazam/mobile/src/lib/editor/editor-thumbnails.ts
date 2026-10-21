import { type Image as AbraPixels, type ImageLike } from '@abra/core/image';
import { AbraImage, ThumbnailPreview } from '@alakazam/mobile';
import { PixelRatio } from 'react-native';
import {
  leafControls,
  scaleBetween,
  thumbnailControls,
  type EditControl,
  type EditSection,
  type GroupControl,
} from '@/src/lib/project/edit-sections';
import { preloadFor } from '@/src/lib/editor/editor-preloads';
import { ownedImage } from '@/src/lib/native/native-image';

/** Display dimensions (dp) of a one-tap control's preview thumbnail. */
export const THUMBNAIL_WIDTH = 112;
export const THUMBNAIL_HEIGHT = 80;
/** How long thumbnails are built before the JS thread is let go to draw the cards made so far. */
const FRAME_BUDGET_MS = 16;

/** The pixel size of a preview card at this screen's density: what the cached thumbnail-size photo is made at. */
export function thumbnailPixels() {
  const density = PixelRatio.get();
  return { width: Math.round(THUMBNAIL_WIDTH * density), height: Math.round(THUMBNAIL_HEIGHT * density) };
}

/**
 * Keeps pixels in Rust, builds one effect at a time, and publishes each completed card as soon as it is ready.
 * Cancellation stops scheduling and prevents retired native components from being updated.
 */
export function buildControlThumbnails(
  /** The edit as it looks now at thumbnail size (see `thumbnailPixels`); this takes ownership and frees it when done. */
  source: AbraImage,
  section: EditSection,
  previous: Record<string, ThumbnailPreview>,
  publish: (key: string, preview: ThumbnailPreview) => void,
  /** The full-size photo the thumbnails are edits of, so a thumb can be told how much smaller its copy is. */
  full: AbraImage | null = null,
  /** The values of the slider that picker items save to, given to their thumbs (see `ToolContext.values`). */
  values?: number[],
  /** Builds start once this settles, so the cards that matter most are not slowed by these on the same thread. */
  after: Promise<unknown> = Promise.resolve(),
) {
  let cancelled = false;
  const { width, height } = thumbnailPixels();
  const done = (async () => {
    await after;
    const started = Date.now();
    let firstCardMs: number | undefined;
    let firstEffectMs: number | undefined;
    let firstComponentMs: number | undefined;
    let completed = 0;
    const cards: string[] = [];
    let next = 0;
    let lastYield = started;
    let succeeded = true;
    try {
      if (cancelled) return false;
      const thumbnailSource = source;
      const render = async () => {
        // A picker's items with a `thumb` are drawn like one-tap controls, each under its own key.
        const controls = thumbnailControls(section.controls).flatMap((control): Exclude<EditControl, GroupControl>[] =>
          control.kind === 'picker'
            ? control.items.flatMap(item => (item.thumb ? [{ kind: 'action', key: item.key, label: item.label, thumb: item.thumb }] : []))
            : [control],
        );
        while (!cancelled && next < controls.length) {
          const control = controls[next++];
          let pixels: AbraImage | undefined;
          let created: ThumbnailPreview | undefined;
          try {
            const effectStarted = Date.now();
            const make = control.kind === 'picker' ? undefined : (control.thumb ?? (control.kind === 'action' ? control.tool : undefined));
            if (make) {
              // The control makes its own thumbnail (its thumb, else its tool), on a copy of the thumbnail-size photo that is its own to change.
              const target = ownedImage(thumbnailSource.clone());
              const photo = target.clone();
              let returned: ImageLike | undefined;
              try {
                // What the control's first slider preloaded (the cutout of the people, say), at the thumbnail's size, so the thumb
                // shows the effect the photo gets. It is only read, never made here, so a thumbnail never starts a model.
                const preloader = leafControls([control]).find(inner => inner.kind === 'slider' && inner.preload);
                const preloaded = preloader?.kind === 'slider' ? preloadFor(preloader, photo) : undefined;
                returned = make.call(control, photo, { scale: scaleBetween(photo, full ?? source), preloaded, values }) as
                  | ImageLike
                  | undefined;
                target.replaceWith(returned ?? photo);
                pixels = target;
              } catch (error) {
                target.uniffiDestroy();
                throw error;
              } finally {
                // They are native images the garbage collector cannot see the size of.
                (photo as AbraPixels).uniffiDestroy();
                if (returned) (returned as AbraPixels).uniffiDestroy();
              }
            } else {
              throw new Error(`Thumbnail control "${control.key}" needs a tool or thumb`);
            }
            if (cancelled) return;
            const effectMs = Date.now() - effectStarted;
            const componentStarted = Date.now();
            const existing = previous[control.key];
            if (existing) {
              existing.setImage(pixels);
              publish(control.key, existing);
            } else {
              created = new ThumbnailPreview(pixels, control.label, width, height);
              publish(control.key, created);
              created = undefined;
            }
            completed++;
            cards.push(`${control.key}: effect ${effectMs}, card ${Date.now() - componentStarted}, at ${Date.now() - started}`);
            if (firstCardMs === undefined) {
              firstCardMs = Date.now() - started;
              firstEffectMs = effectMs;
              firstComponentMs = Date.now() - componentStarted;
            }
          } catch (error) {
            succeeded = false;
            if (!cancelled) console.warn(`[edit] building thumbnail for "${control.key}" threw:`, error);
          } finally {
            created?.uniffiDestroy();
            pixels?.uniffiDestroy();
          }
          // Effects run synchronously on the JS thread, so once a frame's worth of them has built up, let the cards published so far
          // draw. Cheap effects share a frame; a slow one gets its own.
          if (Date.now() - lastYield >= FRAME_BUDGET_MS) {
            await new Promise<void>(resolve => requestAnimationFrame(() => resolve()));
            lastYield = Date.now();
          }
        }
      };
      await render();
      return !cancelled && succeeded;
    } catch (error) {
      if (!cancelled) console.warn('[edit] preparing native thumbnail source threw:', error);
      return false;
    } finally {
      source.uniffiDestroy();
      if (__DEV__ && !cancelled) {
        console.debug('[thumbnails]', section.key, {
          firstCardMs,
          firstEffectMs,
          firstComponentMs,
          totalMs: Date.now() - started,
          completed,
          cards,
        });
      }
    }
  })();
  return {
    done,
    cancel: () => {
      cancelled = true;
    },
  };
}
