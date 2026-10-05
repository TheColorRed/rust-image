import { AbraImage, ThumbnailPreview } from '@alakazam/mobile';
import { PixelRatio } from 'react-native';
import { effectList, type EditSection } from '@/src/lib/edit-sections';
import { ownedImage } from '@/src/lib/native-image';

/** Display dimensions (dp) of a one-tap control's preview thumbnail. */
export const THUMBNAIL_WIDTH = 112;
export const THUMBNAIL_HEIGHT = 80;
const THUMBNAIL_WORKERS = 2;

/**
 * Keeps pixels in Rust, requests at most two effects at once, and publishes each completed card.
 * Cancellation stops scheduling and prevents retired native components from being updated.
 */
export function buildControlThumbnails(
  image: AbraImage,
  section: EditSection,
  previous: Record<string, ThumbnailPreview>,
  publish: (key: string, preview: ThumbnailPreview) => void,
) {
  let cancelled = false;
  const density = PixelRatio.get();
  const width = Math.round(THUMBNAIL_WIDTH * density);
  const height = Math.round(THUMBNAIL_HEIGHT * density);
  const done = (async () => {
    const started = Date.now();
    let sourceMs: number | undefined;
    let firstCardMs: number | undefined;
    let firstEffectMs: number | undefined;
    let firstComponentMs: number | undefined;
    let completed = 0;
    let source: AbraImage | undefined;
    let next = 0;
    let succeeded = true;
    try {
      source = ownedImage(await image.thumbnailSourceAsync(width, height));
      sourceMs = Date.now() - started;
      if (cancelled) return false;
      const thumbnailSource = source;
      const render = async () => {
        while (!cancelled && next < section.controls.length) {
          const control = section.controls[next++];
          let pixels: AbraImage | undefined;
          let created: ThumbnailPreview | undefined;
          try {
            if (control.kind === 'action' && control.apply) {
              throw new Error(`Thumbnail action "${control.key}" needs a native operation or live effect`);
            }
            const effects = control.kind === 'action' && control.live ? effectList(control.live()) : [];
            const operation = control.kind === 'action' ? control.operation : undefined;
            const effectStarted = Date.now();
            pixels = ownedImage(await thumbnailSource.renderThumbnailAsync(effects, operation));
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
      source?.uniffiDestroy();
      if (__DEV__ && !cancelled) {
        console.debug('[thumbnails]', section.key, {
          sourceMs,
          firstCardMs,
          firstEffectMs,
          firstComponentMs,
          totalMs: Date.now() - started,
          completed,
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
