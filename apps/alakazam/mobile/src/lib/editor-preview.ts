import { scaleBetween, type ToolContext } from '@/src/lib/edit-sections';
import { selectedSkinMask } from '@/src/lib/analysis';
import { previewPixels, shrunkOf } from '@/src/lib/editor-pictures';
import { LiveRenderer } from '@/src/lib/live-renderer';
import { people$, skinMask$ } from '@/src/state/analysis';
import { previewBox, previewBox$ } from '@/src/state/gestures';
import { personSelection$ } from '@/src/state/person-selection';
import { liveSurface } from '@/src/state/preview';
import { editBaseImage, editBaseImage$ } from '@/src/state/session';
import { type Image as AbraPixels, type ImageLike } from '@abra/core/image';
import { PixelRatio } from 'react-native';
import { combineLatest, type Subscription } from 'rxjs';
import { distinctUntilChanged, map } from 'rxjs/operators';

/** The editor photo's live renderer: effects drawn over the committed photo while a control is tapped or dragged. */
export const editorPreview = new LiveRenderer(liveSurface);

const sourceIds = new WeakMap<object, number>();
let lastSourceId = 0;

/**
 * Shows what a tool makes of the photo the preview renderer is open over, at the size the preview is drawn at, so it costs what
 * the screen shows. The tool gets an image that is its own to change: it changes it in place and returns nothing, or returns
 * a new image to show instead. `start` is the screen-size photo the tool starts from (the rest of the edit stack), the
 * untouched edit base when left out. Does nothing while the preview has no size.
 */
export function showTool(tool: (photo: ImageLike, context: ToolContext) => ImageLike | void, start?: AbraPixels) {
  const base = editBaseImage.value;
  if (!base || !previewPixels()) return;
  const photo = (start ?? shrunkOf('base', base)).clone();
  let returned: ImageLike | undefined;
  try {
    returned = tool(photo, { scale: scaleBetween(photo, editBaseImage.value) }) as ImageLike | undefined;
    editorPreview.showImage(returned ?? photo);
  } finally {
    // They are native images the garbage collector cannot see the size of, so they are freed even when the tool throws.
    (photo as AbraPixels).uniffiDestroy();
    if (returned) (returned as AbraPixels).uniffiDestroy();
  }
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
  return editorPreview.ensure(`base:${sourceIds.get(base)}`, () => base);
}

/**
 * Keeps the editor's live renderer connected for the whole editing session: it opens as soon as there is an edit base
 * and a preview size, so the first tap or drag only pays for its effects. The preview component handles the handoff
 * after native presentation. Returns the teardown, which lets go of the GPU.
 */
export function startEditorPreview(): Subscription {
  const subscription = combineLatest([editBaseImage$, previewBox$]).subscribe(([base, box]) => {
    if (base && box) ensureBase();
    else editorPreview.release();
  });
  // The skin effects of the preview work on the skin of the selected person, or of everyone.
  subscription.add(
    combineLatest([skinMask$, people$, personSelection$.pipe(map(state => state.selectedId), distinctUntilChanged())]).subscribe(() =>
      editorPreview.setSkin(selectedSkinMask()),
    ),
  );
  subscription.add(() => {
    editorPreview.release();
  });
  return subscription;
}
