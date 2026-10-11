import { RotateCcw, X } from 'lucide-react-native';
import { useEffect, useMemo } from 'react';
import { Alert, Pressable, StyleSheet, View } from 'react-native';
import { useObservable } from 'react-rx';
import { resetEdits } from '@/src/lib/editor-history';
import { useTheme } from '@/src/lib/theme';
import { closeRequested, closeRequested$ } from '@/src/state/commands';
import { timeline$ } from '@/src/state/history';
import { topInset$ } from '@/src/state/preview';

export interface EditorToolbarProps {
  onClose: () => void;
}

/** The close button floating over the photo, and the handler for closing. */
export function EditorToolbar({ onClose }: EditorToolbarProps) {
  const topInset = useObservable(topInset$, 0);
  const hasEdits = useObservable(timeline$, { entries: [], cursor: 0 }).entries.length > 0;
  const { colors } = useTheme();
  const styles = useMemo(() => createStyles(colors), [colors]);

  useEffect(() => {
    const subscription = closeRequested$.subscribe(onClose);
    return () => subscription.unsubscribe();
  }, [onClose]);

  // The button sits on a constant dark scrim (see `overlayButton` in the theme), not the app's
  // chrome, so its icon stays a fixed white regardless of theme.
  const overlayIconColor = '#fff';

  return (
    <View style={[styles.topBar, { top: topInset + 8 }]} pointerEvents="box-none">
      <Pressable style={styles.iconButton} onPress={() => closeRequested.next()} hitSlop={8}>
        <X color={overlayIconColor} size={22} />
      </Pressable>
      {hasEdits ? (
        <Pressable
          style={styles.iconButton}
          hitSlop={8}
          onPress={() =>
            Alert.alert('Reset all edits?', 'The photo goes back to how it was opened, and the history is cleared.', [
              { text: 'Cancel', style: 'cancel' },
              { text: 'Reset', style: 'destructive', onPress: resetEdits },
            ])
          }
        >
          <RotateCcw color={overlayIconColor} size={22} />
        </Pressable>
      ) : null}
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
    iconButton: {
      width: 40,
      height: 40,
      borderRadius: 20,
      backgroundColor: colors.overlayButton,
      alignItems: 'center',
      justifyContent: 'center',
    },
  });
}
