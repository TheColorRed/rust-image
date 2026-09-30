import type { AbraImageLike } from '@alakazam/mobile';
import { ChevronLeft } from 'lucide-react-native';
import { useEffect, useMemo } from 'react';
import { Pressable, StyleSheet, Text, View } from 'react-native';
import { useTheme, type ThemeColors } from '@/src/lib/theme';
import { blemishApplyRequested$, saveRequested } from '@/src/state/commands';
import { isBlemishToolFocused, isBlemishToolFocused$, wasBlemishToolFocused } from '@/src/state/edits';
import { currentImage } from '@/src/state/session';
import { previewBox, previewPressed$, previewScaleValue, previewTranslateValue, springPreview } from '@/src/state/gestures';

/** Screen radii (dp) of the blemish tool's reticle: the inner circle is what gets healed, the
 * outer one is roughly how far around it the tool samples for matching texture. Fixed on screen —
 * pinch-zooming the image underneath is how the user controls how much of the photo they cover. */
export const BLEMISH_INNER_RADIUS = 26;
export const BLEMISH_OUTER_RADIUS = 42;

interface PreviewBox {
  width: number;
  height: number;
}

interface PreviewPoint {
  x: number;
  y: number;
}

/** Renders the two-ring reticle. Fixed on screen — position it outside whatever transformed view
 * carries the pinch-zoom, so it marks a constant spot while the image moves underneath it. */
export function BlemishReticle() {
  const { colors } = useTheme();
  const styles = useMemo(() => createStyles(colors), [colors]);
  return (
    <View style={styles.reticle} pointerEvents="none">
      <View style={[styles.ring, styles.ringOuter]} />
      <View style={[styles.ring, styles.ringInner]} />
    </View>
  );
}

export interface BlemishControlsProps {
  onBack: () => void;
  onApply: () => void;
}

/** The controls-row UI shown while the blemish tool is focused: a back chip, a hint, and the
 * button that actually runs the removal at the reticle's current spot. */
export function BlemishControls({ onBack, onApply }: BlemishControlsProps) {
  const { colors } = useTheme();
  const styles = useMemo(() => createStyles(colors), [colors]);
  return (
    <View style={styles.controlRow}>
      <View style={styles.topRow}>
        <Pressable onPress={onBack} hitSlop={8} style={styles.backChip}>
          <ChevronLeft color={colors.textPrimary} size={18} />
        </Pressable>
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
        const image = currentImage.value;
        const box = previewBox.value;
        if (!image || !box) return;
        const target = computeBlemishTarget(image, box, previewScaleValue.value, previewTranslateValue.value);
        if (!target) return;
        image.removeBlemish(target.x, target.y, target.radius);
        saveRequested.next();
      }),

      // Double-tapping a spot pans it to the reticle's center, rather than requiring a drag to line
      // it up — the same math and pan bounds as the pinch gesture. `locationX/Y` is relative to the
      // preview's untransformed touch target, which lines up with previewBox-local coordinates.
      previewPressed$.subscribe(event => {
        const box = previewBox.value;
        if (!isBlemishToolFocused() || !box) return;
        const { locationX, locationY, pageX, pageY } = event.nativeEvent;
        const now = Date.now();
        const last = lastTap;
        lastTap = { time: now, x: pageX, y: pageY };
        if (!last || now - last.time > 300 || Math.hypot(pageX - last.x, pageY - last.y) > 40) return;
        lastTap = null;
        springPreview(computeCenteringTranslate({ x: locationX, y: locationY }, box, previewScaleValue.value));
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

  // Reversing "screen = center + scale*(local-center) + translate" for a screen point that IS the
  // center gives local = center - translate/scale.
  const localX = box.width / 2 - translate.x / scale;
  const localY = box.height / 2 - translate.y / scale;

  // `fit="contain"` letterboxes the image within the box; map through that before going from
  // normalized (0..1) position to actual image pixels.
  const containScale = Math.min(box.width / imgWidth, box.height / imgHeight);
  const displayedWidth = imgWidth * containScale;
  const displayedHeight = imgHeight * containScale;
  const offsetX = (box.width - displayedWidth) / 2;
  const offsetY = (box.height - displayedHeight) / 2;
  const u = (localX - offsetX) / displayedWidth;
  const v = (localY - offsetY) / displayedHeight;
  if (u < 0 || u > 1 || v < 0 || v > 1) return null; // reticle is over the letterboxed margin, not the photo

  const displayScale = containScale * scale;
  const radius = Math.min(Math.min(imgWidth, imgHeight) / 2, Math.max(2, BLEMISH_INNER_RADIUS / displayScale));
  return { x: u * imgWidth, y: v * imgHeight, radius };
}

/**
 * The pan translate that would bring `point` (in previewBox-local coordinates — the same space
 * `event.nativeEvent.locationX/Y` reports on the untransformed preview touch target) to the
 * screen center, clamped to the same pan bounds pinch-zooming itself respects.
 * - `point`: the tapped spot, in previewBox-local coordinates.
 * - `box`: the preview's on-screen size.
 * - `scale`: the pinch-zoom's current scale (unchanged by this — only pans, doesn't zoom).
 */
export function computeCenteringTranslate(point: PreviewPoint, box: PreviewBox, scale: number): PreviewPoint {
  // Match the gesture's overscroll range so a double tap can center a blemish
  // all the way at an image edge.
  const maxTranslateX = (scale * box.width) / 2;
  const maxTranslateY = (scale * box.height) / 2;
  const rawX = -scale * (point.x - box.width / 2);
  const rawY = -scale * (point.y - box.height / 2);
  return {
    x: Math.min(maxTranslateX, Math.max(-maxTranslateX, rawX)),
    y: Math.min(maxTranslateY, Math.max(-maxTranslateY, rawY)),
  };
}

function createStyles(colors: ThemeColors) {
  return StyleSheet.create({
    reticle: {
      position: 'absolute',
      top: 0,
      left: 0,
      right: 0,
      bottom: 0,
      alignItems: 'center',
      justifyContent: 'center',
    },
    ring: { position: 'absolute', borderRadius: 999, borderWidth: 2 },
    ringOuter: {
      width: BLEMISH_OUTER_RADIUS * 2,
      height: BLEMISH_OUTER_RADIUS * 2,
      borderColor: 'rgba(255,255,255,0.5)',
    },
    ringInner: { width: BLEMISH_INNER_RADIUS * 2, height: BLEMISH_INNER_RADIUS * 2, borderColor: colors.accent },

    controlRow: { alignItems: 'center', gap: 8, paddingHorizontal: 16 },
    topRow: { flexDirection: 'row', alignItems: 'center', gap: 10, alignSelf: 'stretch' },
    backChip: {
      width: 28,
      height: 28,
      borderRadius: 14,
      backgroundColor: colors.chip,
      alignItems: 'center',
      justifyContent: 'center',
    },
    hint: { flex: 1, color: colors.textSecondary, fontSize: 12 },
    applyButton: { paddingHorizontal: 20, paddingVertical: 10, borderRadius: 18, backgroundColor: colors.accent },
    applyButtonText: { color: '#fff', fontSize: 14, fontWeight: '700' },
  });
}
