import { type Image as AbraPixels, type ImageLike } from '@abra/core/image';
import { AbraImage, ThumbnailPreview } from '@alakazam/mobile';
import { PixelRatio } from 'react-native';
import { leafControls, scaleBetween, type EditSection } from '@/src/lib/edit-sections';
import { ownedImage } from '@/src/lib/native-image';

/** Display dimensions (dp) of a one-tap control's preview thumbnail. */
export const THUMBNAIL_WIDTH = 112;
export const THUMBNAIL_HEIGHT = 80;
const THUMBNAIL_WORKERS = 2;

/** The pixel size of a preview card at this screen's density: what the cached thumbnail-size photo is made at. */
export function thumbnailPixels() {
  const density = PixelRatio.get();
  return { width: Math.round(THUMBNAIL_WIDTH * density), height: Math.round(THUMBNAIL_HEIGHT * density) };
}

/**
 * Keeps pixels in Rust, requests at most two effects at once, and publishes each completed card.
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
) {
  let cancelled = false;
  const { width, height } = thumbnailPixels();
  const done = (async () => {
    const started = Date.now();
    let firstCardMs: number | undefined;
    let firstEffectMs: number | undefined;
    let firstComponentMs: number | undefined;
    let completed = 0;
    const cards: string[] = [];
    let next = 0;
    let succeeded = true;
    try {
      if (cancelled) return false;
      const thumbnailSource = source;
      const render = async () => {
        const controls = leafControls(section.controls);
        while (!cancelled && next < controls.length) {
          const control = controls[next++];
          let pixels: AbraImage | undefined;
          let created: ThumbnailPreview | undefined;
          try {
            const effectStarted = Date.now();
            const make = control.thumb ?? (control.kind === 'action' ? control.tool : undefined);
            if (make) {
              // The control makes its own thumbnail (its thumb, else its tool), on a copy of the thumbnail-size photo that is its own to change.
              const target = ownedImage(thumbnailSource.clone());
              const photo = target.clone();
              let returned: ImageLike | undefined;
              try {
                returned = make.call(control, photo, { scale: scaleBetween(photo, full ?? source) }) as ImageLike | undefined;
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
        }
      };
      await Promise.all(Array.from({ length: THUMBNAIL_WORKERS }, render));
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
