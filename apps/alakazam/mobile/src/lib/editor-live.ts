import { PixelRatio } from 'react-native';
import { combineLatest, type Subscription } from 'rxjs';
import { LiveRenderer } from '@/src/lib/live-renderer';
import { previewBox, previewBox$ } from '@/src/state/gestures';
import { liveSurface } from '@/src/state/preview';
import { editBaseImage, editBaseImage$ } from '@/src/state/session';

/** The editor photo's live renderer: effects drawn over the committed photo while a control is tapped or dragged. */
export const editorLive = new LiveRenderer(liveSurface);

const sourceIds = new WeakMap<object, number>();
let lastSourceId = 0;

/** The pixel size the editor's live frames are rendered at: the preview box at the screen's density. */
export function previewPixels(): { width: number; height: number } | null {
  const box = previewBox.value;
  if (!box) return null;
  const ratio = PixelRatio.get();
  return { width: Math.round(box.width * ratio), height: Math.round(box.height * ratio) };
}

/**
 * Makes sure the editor's live renderer is open over the untouched edit base, which is what every chain of effects is
 * applied to. Reuses the open preview when the base and size have not changed. Returns whether it is open.
 */
export function ensureBase(): boolean {
  const base = editBaseImage.value;
  const size = previewPixels();
  if (!base || !size) return false;
  if (!sourceIds.has(base)) sourceIds.set(base, ++lastSourceId);
  return editorLive.ensure(`base:${sourceIds.get(base)}`, () => base);
}

/**
 * Keeps the editor's live renderer connected for the whole editing session: it opens as soon as there is an edit base
 * and a preview size, so the first tap or drag only pays for its effects. The preview component handles the handoff
 * after native presentation. Returns the teardown, which lets go of the GPU.
 */
export function startEditorLive(): Subscription {
  const subscription = combineLatest([editBaseImage$, previewBox$]).subscribe(([base, box]) => {
    if (base && box) ensureBase();
    else editorLive.release();
  });
  subscription.add(() => {
    editorLive.release();
  });
  return subscription;
}
