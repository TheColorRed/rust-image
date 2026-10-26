import type { AbraImageLike } from '@alakazam/mobile';
import { useEffect, useMemo } from 'react';
import { useObservable } from 'react-rx';
import { PixelRatio, Pressable, StyleSheet, Text, View } from 'react-native';
import { Color } from '@abra/core/color';
import { Brush } from '@abra/core/drawing';
import { Area } from '@abra/core/geometry';
import { editorGizmos, editorGizmos$ } from '@/src/state/gizmos';
import { BLEMISH_TOOL_KEY } from '@/src/lib/project/edit-sections/keys';
import { owning } from '@/src/lib/native/native-handles';
import { useTheme, type ThemeColors } from '@/src/lib/ui/theme';
import { computePreviewImagePoint, unrotatePreviewPoint, type PreviewPoint } from '@/src/lib/editor/preview-coordinates';
import { applyPluginTool } from '@/src/lib/plugins/plugin-controls';
import { blemishApplyRequested$ } from '@/src/state/commands';
import { isBlemishToolFocused, isBlemishToolFocused$, wasBlemishToolFocused } from '@/src/state/edits';
import { historyReady } from '@/src/state/project-session';
import { flushFullReplay } from '@/src/lib/editor/editor-replay';
import { currentImage } from '@/src/state/session';
import {
  previewBox,
  previewBox$,
  previewPressed$,
  previewRotationValue,
  previewScaleValue,
  previewTranslateValue,
  springPreview,
  type PreviewBox,
} from '@/src/state/gestures';

/** Screen radii (dp) of the blemish tool's reticle: the inner circle is what gets healed, the
 * outer one is roughly how far around it the tool samples for matching texture. Fixed on screen —
 * pinch-zooming the image underneath is how the user controls how much of the photo they cover. */
export const BLEMISH_INNER_RADIUS = 26;
export const BLEMISH_OUTER_RADIUS = 42;
/** How thick the reticle's rings are, in dp. */
const RETICLE_LINE = 2;

/** Renders the two-ring reticle. Fixed on screen — position it outside whatever transformed view
 * carries the pinch-zoom, so it marks a constant spot while the image moves underneath it. */
export function BlemishReticle() {
  const { colors } = useTheme();
  // The photo is zoomed and turned about the middle of the preview box, which can be taller than what is visible (a control
  // opening a taller panel covers the bottom of it), so the reticle marks that middle and not the middle of what is visible.
  const box = useObservable(previewBox$, previewBox.value);
  const gizmos = useObservable(editorGizmos$, editorGizmos.value);
  // The library draws the two rings on the gizmos view over the photo. Their sizes are in the view's pixels, and each ring's edge
  // runs along its radius, as the border of a view of that size does.
  useEffect(() => {
    if (!gizmos || !box) return;
    const density = PixelRatio.get();
    const center = { x: (box.width * density) / 2, y: (box.height * density) / 2 };
    const width = Math.round(RETICLE_LINE * density);
    const names = owning(keep => {
      const outer = keep(new Brush().withColor(Color.white()).withSize(width).withHardness(1).withOpacity(0.5) as Brush);
      const inner = keep(new Brush().withColor(Color.fromHexString(colors.accent)).withSize(width).withHardness(1) as Brush);
      const rings = [
        ['blemish-outer', BLEMISH_OUTER_RADIUS, outer],
        ['blemish-inner', BLEMISH_INNER_RADIUS, inner],
      ] as const;
      for (const [name, radius, brush] of rings) {
        gizmos.setOutline(name, keep(Area.circle(center, radius * density - width / 2) as Area), brush);
      }
      return rings.map(([name]) => name);
    });
    return () => {
      for (const name of names) gizmos.remove(name);
    };
  }, [gizmos, box, colors.accent]);
  return null;
}

export interface BlemishControlsProps {
  onApply: () => void;
}

/** The controls-row UI shown while the blemish tool is focused: a hint, and the
 * button that actually runs the removal at the reticle's current spot. */
export function BlemishControls({ onApply }: BlemishControlsProps) {
  const { colors } = useTheme();
  const styles = useMemo(() => createStyles(colors), [colors]);
  return (
    <View style={styles.controlRow}>
      <View style={styles.topRow}>
        <Text style={styles.hint}>Pinch to fit the blemish in the inner circle, or double-tap it</Text>
      </View>
      <Pressable style={styles.applyButton} onPress={onApply}>
        <Text style={styles.applyButtonText}>Remove Blemish</Text>
      </Pressable>
    </View>
  );
}

/**
 * Wires up everything the blemish tool does on the editor controller: running the removal at the
 * reticle, double-tap-to-center on the preview, and restoring ordinary pan bounds on leaving.
 */
export function BlemishToolController() {
  useEffect(() => {
    let lastTap: { time: number; x: number; y: number } | null = null;
    const subscriptions = [
      blemishApplyRequested$.subscribe(() => {
        // The blemish is healed on the full-size photo, so it must have this edit's full-size version first.
        flushFullReplay();
        const image = currentImage.value;
        const box = previewBox.value;
        // Until the history is built the image is a project's saved picture, and healing it would be lost.
        if (!image || !box || !historyReady.value) return;
        const target = computeBlemishTarget(image, box, previewScaleValue.value, previewTranslateValue.value);
        if (!target) return;
        applyPluginTool(BLEMISH_TOOL_KEY, target);
      }),

      // Double-tapping a spot pans it to the reticle's center, rather than requiring a drag to line
      // it up — the same math and pan bounds as the pinch gesture.
      previewPressed$.subscribe(event => {
        const box = previewBox.value;
        if (!isBlemishToolFocused() || !box) return;
        const { x, y, pageX, pageY } = event;
        const now = Date.now();
        const last = lastTap;
        lastTap = { time: now, x: pageX, y: pageY };
        if (!last || now - last.time > 300 || Math.hypot(pageX - last.x, pageY - last.y) > 40) return;
        lastTap = null;
        // The tap is on the screen, where the photo is turned; the pan is in the photo's own axes.
        springPreview(
          computeCenteringTranslate(
            unrotatePreviewPoint({ x, y }, box, previewRotationValue.value),
            box,
            previewScaleValue.value,
            previewTranslateValue.value,
          ),
        );
      }),

      // Blemish mode permits overscroll so an edge can reach the fixed reticle. Restore the ordinary
      // preview bounds when leaving the tool, without throwing away an in-bounds zoomed viewport.
      // At 1x those bounds are zero, so this springs fully back to center.
      isBlemishToolFocused$.subscribe(focused => {
        const was = wasBlemishToolFocused.value;
        wasBlemishToolFocused.next(focused);
        const box = previewBox.value;
        if (!was || focused || !box) return;
        const scale = previewScaleValue.value;
        const maxX = ((scale - 1) * box.width) / 2;
        const maxY = ((scale - 1) * box.height) / 2;
        const { x, y } = previewTranslateValue.value;
        springPreview({ x: Math.min(maxX, Math.max(-maxX, x)), y: Math.min(maxY, Math.max(-maxY, y)) });
      }),
    ];
    return () => subscriptions.forEach(subscription => subscription.unsubscribe());
  }, []);

  return null;
}

/**
 * Where the reticle (fixed at the screen center) currently points to, in image pixels, and the
 * heal radius that corresponds to its fixed on-screen size at the current zoom — so zooming in
 * shrinks the effective radius, letting a heavy zoom isolate a small blemish precisely. Returns
 * `null` when the reticle is over the letterboxed margin around the image, not the photo itself.
 * - `image`: the current working image (for its pixel dimensions).
 * - `box`: the preview's on-screen size.
 * - `scale`, `translate`: the pinch-zoom's current values (see the edit screen's PanResponder).
 */
export function computeBlemishTarget(
  image: AbraImageLike,
  box: PreviewBox,
  scale: number,
  translate: PreviewPoint,
): { x: number; y: number; radius: number } | null {
  const imgWidth = image.width();
  const imgHeight = image.height();
  if (imgWidth === 0 || imgHeight === 0) return null;

  const point = computePreviewImagePoint(imgWidth, imgHeight, box, scale, translate, {
    x: box.width / 2,
    y: box.height / 2,
  });
  if (!point) return null;
  const containScale = Math.min(box.width / imgWidth, box.height / imgHeight);

  const displayScale = containScale * scale;
  const radius = Math.min(Math.min(imgWidth, imgHeight) / 2, Math.max(2, BLEMISH_INNER_RADIUS / displayScale));
  return { ...point, radius };
}

/**
 * The pan translate that would bring `point` (in untransformed previewBox-local coordinates) to the
 * screen center, clamped to the same pan bounds pinch-zooming itself respects.
 * - `point`: the tapped spot, in previewBox-local coordinates.
 * - `box`: the preview's on-screen size.
 * - `scale`: the pinch-zoom's current scale (unchanged by this — only pans, doesn't zoom).
 * - `translate`: the pan now, since the tapped spot is where the photo is now and not where it started.
 */
export function computeCenteringTranslate(point: PreviewPoint, box: PreviewBox, scale: number, translate: PreviewPoint): PreviewPoint {
  // Match the gesture's overscroll range so a double tap can center a blemish
  // all the way at an image edge.
  const maxTranslateX = (scale * box.width) / 2;
  const maxTranslateY = (scale * box.height) / 2;
  const rawX = translate.x - (point.x - box.width / 2);
  const rawY = translate.y - (point.y - box.height / 2);
  return {
    x: Math.min(maxTranslateX, Math.max(-maxTranslateX, rawX)),
    y: Math.min(maxTranslateY, Math.max(-maxTranslateY, rawY)),
  };
}

function createStyles(colors: ThemeColors) {
  return StyleSheet.create({
    controlRow: { alignItems: 'center', gap: 8, paddingHorizontal: 16 },
    topRow: { flexDirection: 'row', alignItems: 'center', gap: 10, alignSelf: 'stretch' },
    hint: { flex: 1, color: colors.textSecondary, fontSize: 12 },
    applyButton: { paddingHorizontal: 20, paddingVertical: 10, borderRadius: 18, backgroundColor: colors.accent },
    applyButtonText: { color: '#fff', fontSize: 14, fontWeight: '700' },
  });
}
