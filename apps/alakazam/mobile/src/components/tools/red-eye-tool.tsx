import { ChevronLeft } from 'lucide-react-native';
import { useEffect, useMemo } from 'react';
import { Pressable, StyleSheet, Text, View } from 'react-native';
import { useObservable } from 'react-rx';
import { BehaviorSubject } from 'rxjs';
import { missingModels } from '@/src/lib/ai-model-availability';
import { commitRetouch } from '@/src/lib/editor/editor-history';
import { flushFullReplay } from '@/src/lib/editor/editor-replay';
import { errorMessage } from '@/src/lib/error-message';
import { computePreviewImagePoint } from '@/src/lib/preview-coordinates';
import { RED_EYE_MODELS, applyRedEyeStep, type RedEyeChange } from '@/src/lib/red-eye';
import { useTheme, type ThemeColors } from '@/src/lib/theme';
import { aiModelsRequested } from '@/src/state/ai-models';
import { redEyeAutoRequested$ } from '@/src/state/commands';
import { isRedEyeToolFocused } from '@/src/state/edits';
import { previewBox, previewPressed$, previewScaleValue, previewTranslateValue } from '@/src/state/gestures';
import { historyReady } from '@/src/state/project-session';
import { busy, currentImage } from '@/src/state/session';

/**
 * What the tool last had to say, such as that a tap found no red-eye. It is shown in the controls and clears itself, because
 * a dialog would take the screen's focus in the middle of a touch and stop the next pinch from working.
 */
const message = new BehaviorSubject<string | null>(null);
const message$ = message.asObservable();
const MESSAGE_MS = 4000;

export interface RedEyeControlsProps {
  onBack: () => void;
  onAuto: () => void;
}

/** The controls-row UI shown while the red-eye tool is focused: a back chip, a hint, and the button for automatic removal. */
export function RedEyeControls({ onBack, onAuto }: RedEyeControlsProps) {
  const { colors } = useTheme();
  const styles = useMemo(() => createStyles(colors), [colors]);
  const said = useObservable(message$, null);
  return (
    <View style={styles.controlRow}>
      <View style={styles.topRow}>
        <Pressable onPress={onBack} hitSlop={8} style={styles.backChip}>
          <ChevronLeft color={colors.textPrimary} size={18} />
        </Pressable>
        <Text style={said ? styles.message : styles.hint}>
          {said ?? 'Tap each red eye, or let the AI find them. Pinch to zoom in first for a closer tap.'}
        </Text>
      </View>
      <Pressable style={styles.autoButton} onPress={onAuto}>
        <Text style={styles.autoButtonText}>Auto Removal</Text>
      </Pressable>
    </View>
  );
}

/**
 * Wires up everything the red-eye tool does on the editor: fixing the eye that is tapped on the preview (which needs no AI
 * model), and the Auto Removal button (which does, and offers to download them first).
 */
export function RedEyeToolController() {
  useEffect(() => {
    let clearTimer: ReturnType<typeof setTimeout> | undefined;
    const say = (text: string) => {
      message.next(text);
      clearTimeout(clearTimer);
      clearTimer = setTimeout(() => message.next(null), MESSAGE_MS);
    };

    /** Applies a red-eye fix to the full-size photo and records it as a step, or tells the user why nothing changed. */
    const fix = (change: RedEyeChange, nothingFound: string) => {
      // The fix is made on the full-size photo, so it must have this edit's full-size version first.
      flushFullReplay();
      const image = currentImage.value;
      // Until the history is built the image is a project's saved picture, and a fix made on it would be lost.
      if (!image || !historyReady.value) return;
      try {
        if (!applyRedEyeStep(image, change)) {
          say(nothingFound);
          return;
        }
        message.next(null);
        commitRetouch(change);
      } catch (error) {
        say(`Could not fix red-eye: ${errorMessage(error)}`);
      }
    };

    const runAuto = () => {
      busy.next(true);
      // Let the spinner draw before the models start, which hold up the screen for a moment.
      setTimeout(() => {
        try {
          fix({ type: 'red-eye-auto' }, 'No red-eye found. Tap each eye to fix it yourself.');
        } finally {
          busy.next(false);
        }
      }, 50);
    };

    const subscriptions = [
      previewPressed$.subscribe(event => {
        const box = previewBox.value;
        const image = currentImage.value;
        if (!isRedEyeToolFocused() || !box || !image) return;
        const point = computePreviewImagePoint(
          image.width(),
          image.height(),
          box,
          previewScaleValue.value,
          previewTranslateValue.value,
          event,
        );
        if (!point) return;
        fix({ type: 'red-eye', ...point }, 'No red-eye found there. Tap closer to the red pupil, or pinch to zoom in and tap again.');
      }),

      redEyeAutoRequested$.subscribe(() => {
        // Automatic removal needs the AI models. Without them, offer to download them, and run once they are here.
        if (missingModels(RED_EYE_MODELS).length > 0) {
          aiModelsRequested.next({ title: 'Auto red-eye removal', roles: RED_EYE_MODELS, onReady: runAuto });
          return;
        }
        runAuto();
      }),
    ];
    return () => {
      clearTimeout(clearTimer);
      message.next(null);
      subscriptions.forEach(subscription => subscription.unsubscribe());
    };
  }, []);

  return null;
}

function createStyles(colors: ThemeColors) {
  return StyleSheet.create({
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
    message: { flex: 1, color: colors.danger, fontSize: 12, fontWeight: '600' },
    autoButton: { paddingHorizontal: 20, paddingVertical: 10, borderRadius: 18, backgroundColor: colors.accent },
    autoButtonText: { color: '#fff', fontSize: 14, fontWeight: '700' },
  });
}
