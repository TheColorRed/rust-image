import { Check, RotateCcw, X } from 'lucide-react-native';
import { useEffect, useMemo } from 'react';
import { Pressable, StyleSheet, View } from 'react-native';
import { useObservable } from 'react-rx';
import { resetDraft, saveDraft } from '@/src/lib/editor-history';
import { useTheme } from '@/src/lib/theme';
import {
  closeRequested,
  closeRequested$,
  resetRequested,
  resetRequested$,
  saveRequested,
  saveRequested$,
} from '@/src/state/commands';
import { adjustments$, appliedActions$ } from '@/src/state/edits';
import { topInset$ } from '@/src/state/preview';

export interface EditorToolbarProps {
  onClose: () => void;
}

/** Close / reset / save buttons floating over the photo, and the handlers for those commands. */
export function EditorToolbar({ onClose }: EditorToolbarProps) {
  const topInset = useObservable(topInset$, 0);
  const adjustments = useObservable(adjustments$, {});
  const appliedActions = useObservable(appliedActions$, []);
  const dirty = appliedActions.length > 0 || Object.values(adjustments).some(value => value !== 0);
  const { colors } = useTheme();
  const styles = useMemo(() => createStyles(colors), [colors]);

  useEffect(() => {
    const subscriptions = [
      closeRequested$.subscribe(onClose),
      resetRequested$.subscribe(resetDraft),
      saveRequested$.subscribe(saveDraft),
    ];
    return () => subscriptions.forEach(subscription => subscription.unsubscribe());
  }, [onClose]);

  // The buttons sit on a constant dark scrim (see `overlayButton` in the theme), not the app's
  // chrome, so their icons stay a fixed white regardless of theme.
  const overlayIconColor = '#fff';
  const disabledIconColor = colors.textTertiary;

  return (
    <View style={[styles.topBar, { top: topInset + 8 }]} pointerEvents="box-none">
      <Pressable style={styles.iconButton} onPress={() => closeRequested.next()} hitSlop={8}>
        <X color={overlayIconColor} size={22} />
      </Pressable>
      <View style={styles.right}>
        <Pressable style={styles.iconButton} onPress={() => resetRequested.next()} disabled={!dirty} hitSlop={8}>
          <RotateCcw color={dirty ? overlayIconColor : disabledIconColor} size={20} />
        </Pressable>
        <Pressable
          style={[styles.iconButton, styles.saveButton, !dirty && styles.saveButtonDisabled]}
          onPress={() => saveRequested.next()}
          disabled={!dirty}
          hitSlop={8}
        >
          <Check color={dirty ? colors.background : disabledIconColor} size={20} />
        </Pressable>
      </View>
    </View>
  );
}

function createStyles(colors: ReturnType<typeof useTheme>['colors']) {
  return StyleSheet.create({
    topBar: {
      position: 'absolute',
      left: 12,
      right: 12,
      flexDirection: 'row',
      alignItems: 'center',
      justifyContent: 'space-between',
    },
    right: { flexDirection: 'row', gap: 10 },
    iconButton: {
      width: 40,
      height: 40,
      borderRadius: 20,
      backgroundColor: colors.overlayButton,
      alignItems: 'center',
      justifyContent: 'center',
    },
    saveButton: { backgroundColor: colors.textPrimary },
    saveButtonDisabled: { backgroundColor: colors.overlayButton },
  });
}
