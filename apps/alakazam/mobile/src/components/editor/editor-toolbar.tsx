import { CircleQuestionMark, Library, RotateCcw, X } from 'lucide-react-native';
import { useEffect, useMemo, useState } from 'react';
import { Alert, Pressable, ScrollView, StyleSheet, View } from 'react-native';
import { useObservable } from 'react-rx';
import { ImageInfoDialog } from '@/src/components/editor/image-info-dialog';
import { PresetDialog } from '@/src/components/editor/preset-dialog';
import { type EditablePhoto } from '@/src/lib/edit-source';
import { resetEdits } from '@/src/lib/editor/editor-history';
import { editorPreview } from '@/src/lib/editor/editor-preview';
import { useTheme } from '@/src/lib/theme';
import { closeRequested, closeRequested$ } from '@/src/state/commands';
import { timeline$ } from '@/src/state/history';
import { topInset$ } from '@/src/state/preview';

export interface EditorToolbarProps {
  photo: EditablePhoto;
  onClose: () => void;
}

/** The buttons floating over the photo: close on the left, and on the right the photo's information and the reset of the edits. */
export function EditorToolbar({ photo, onClose }: EditorToolbarProps) {
  const topInset = useObservable(topInset$, 0);
  const hasEdits = useObservable(timeline$, { entries: [], cursor: 0 }).entries.length > 0;
  const { colors } = useTheme();
  const [infoOpen, setInfoOpen] = useState(false);
  const [presetsOpen, setPresetsOpen] = useState(false);
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
      {/* The close button stays put; the buttons beside it scroll sideways when the screen is too narrow for them all. */}
      <ScrollView horizontal showsHorizontalScrollIndicator={false} style={styles.rightScroll} contentContainerStyle={styles.right}>
        <Pressable accessibilityLabel="Presets" style={styles.iconButton} onPress={() => setPresetsOpen(true)} hitSlop={8}>
          <Library color={overlayIconColor} size={22} />
        </Pressable>
        <Pressable accessibilityLabel="Photo information" style={styles.iconButton} onPress={() => setInfoOpen(true)} hitSlop={8}>
          <CircleQuestionMark color={overlayIconColor} size={22} />
        </Pressable>
        {hasEdits ? (
          <Pressable
            accessibilityLabel="Reset all edits"
            style={styles.iconButton}
            hitSlop={8}
            onPress={() =>
              Alert.alert('Reset all edits?', 'The photo goes back to how it was opened, and the history is cleared.', [
                { text: 'Cancel', style: 'cancel' },
                {
                  text: 'Reset',
                  style: 'destructive',
                  // The live view still shows the last tool's result over the photo, so it is taken off with the edits.
                  onPress: () => {
                    editorPreview.hide();
                    resetEdits();
                  },
                },
              ])
            }
          >
            <RotateCcw color={overlayIconColor} size={22} />
          </Pressable>
        ) : null}
      </ScrollView>
      <ImageInfoDialog photo={photo} visible={infoOpen} onClose={() => setInfoOpen(false)} />
      <PresetDialog visible={presetsOpen} onClose={() => setPresetsOpen(false)} />
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
    rightScroll: { flexGrow: 0, flexShrink: 1, marginLeft: 12 },
    right: {
      flexDirection: 'row',
      alignItems: 'center',
      gap: 12,
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
