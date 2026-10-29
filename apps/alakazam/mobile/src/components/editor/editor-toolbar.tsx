import Bug from 'lucide-react-native/icons/bug';
import CircleQuestionMark from 'lucide-react-native/icons/circle-question-mark';
import Library from 'lucide-react-native/icons/library';
import RotateCcw from 'lucide-react-native/icons/rotate-ccw';
import Save from 'lucide-react-native/icons/save';
import Share2 from 'lucide-react-native/icons/share-2';
import X from 'lucide-react-native/icons/x';
import { useEffect, useMemo, useState } from 'react';
import type { ReactNode } from 'react';
import { Alert, Pressable, ScrollView, StyleSheet, Text, View } from 'react-native';
import { useObservable } from 'react-rx';
import { DeveloperDialog } from '@/src/components/editor/developer-dialog';
import { ImageInfoDialog } from '@/src/components/editor/image-info-dialog';
import { PresetDialog } from '@/src/components/editor/preset-dialog';
import { type EditablePhoto } from '@/src/lib/editor/edit-source';
import { resetEdits } from '@/src/lib/editor/editor-history';
import { saveEditedPhotoToGallery, shareEditedPhoto } from '@/src/lib/editor/editor-session';
import { ExportDialog, type ExportDialogItem } from '@/src/components/export-dialog';
import type { ExportFormat } from '@/src/state/settings';
import { errorMessage } from '@/src/lib/utils/error-message';
import { editorPreview } from '@/src/lib/editor/editor-preview';
import { useTheme } from '@/src/lib/ui/theme';
import { closeRequested, closeRequested$ } from '@/src/state/commands';
import { timeline$ } from '@/src/state/history';
import { adjustments, appliedActions } from '@/src/state/edits';

export interface EditorToolbarProps {
  photo?: EditablePhoto;
  onClose: () => void;
  title?: string;
  rightActions?: ReactNode;
}

/** The buttons floating over the photo: close on the left, and on the right the photo's information and the reset of the edits. */
export function EditorToolbar({ photo, onClose, title, rightActions }: EditorToolbarProps) {
  // A reopened project restores its recipe but not its undo timeline. Include the live edit state
  // so Reset remains available for saved adjustments and actions after reopening.
  const hasTimelineEdits = useObservable(timeline$, { entries: [], cursor: 0 }).entries.length > 0;
  const hasAdjustmentEdits = Object.keys(useObservable(adjustments, {})).length > 0;
  const hasActionEdits = useObservable(appliedActions, []).length > 0;
  const hasEdits = hasTimelineEdits || hasAdjustmentEdits || hasActionEdits;
  const { colors } = useTheme();
  const [infoOpen, setInfoOpen] = useState(false);
  const [presetsOpen, setPresetsOpen] = useState(false);
  const [developerOpen, setDeveloperOpen] = useState(false);
  const [sharing, setSharing] = useState(false);
  const [saveOpen, setSaveOpen] = useState(false);
  const exportItems = useMemo(
    () => photo ? [{ id: 'editor-photo', uri: photo.uri, name: photo.fileName ?? photo.project?.name ?? 'photo.jpg' }] : [],
    [photo?.fileName, photo?.project?.name, photo?.uri],
  );
  const saveCurrentPhoto = async (_item: ExportDialogItem, fileName: string, format: ExportFormat, quality: number) =>
    saveEditedPhotoToGallery(fileName, format, quality);
  const styles = useMemo(() => createStyles(colors), [colors]);

  useEffect(() => {
    const subscription = closeRequested$.subscribe(onClose);
    return () => subscription.unsubscribe();
  }, [onClose]);

  // The button sits on a constant dark scrim (see `overlayButton` in the theme), not the app's
  // chrome, so its icon stays a fixed white regardless of theme.
  const overlayIconColor = '#fff';

  return (
    <View style={[styles.topBar, { top: 8 }]} pointerEvents="box-none">
      <Pressable style={styles.iconButton} onPress={() => closeRequested.next()} hitSlop={8}>
        <X color={overlayIconColor} size={22} />
      </Pressable>
      {title ? <View style={styles.titleWrap} pointerEvents="none"><Text style={styles.title}>{title}</Text></View> : null}
      {/* The close button stays put; the buttons beside it scroll sideways when the screen is too narrow for them all. */}
      <ScrollView horizontal showsHorizontalScrollIndicator={false} style={styles.rightScroll} contentContainerStyle={styles.right}>
        {photo ? <Pressable
          accessibilityLabel="Save to device"
          style={styles.iconButton}
          onPress={() => setSaveOpen(true)}
          hitSlop={8}
        >
          <Save color={overlayIconColor} size={22} />
        </Pressable> : null}
        {photo ? <Pressable
          accessibilityLabel="Share"
          style={[styles.iconButton, sharing && styles.disabled]}
          disabled={sharing}
          hitSlop={8}
          onPress={() => {
            setSharing(true);
            shareEditedPhoto()
              .catch(error => Alert.alert('Could not share the photo', errorMessage(error)))
              .finally(() => setSharing(false));
          }}
        >
          <Share2 color={overlayIconColor} size={22} />
        </Pressable> : null}
        {photo ? <Pressable accessibilityLabel="Presets" style={styles.iconButton} onPress={() => setPresetsOpen(true)} hitSlop={8}>
          <Library color={overlayIconColor} size={22} />
        </Pressable> : null}
        {photo ? <Pressable accessibilityLabel="Photo information" style={styles.iconButton} onPress={() => setInfoOpen(true)} hitSlop={8}>
          <CircleQuestionMark color={overlayIconColor} size={22} />
        </Pressable> : null}
        {__DEV__ ? (
          <Pressable accessibilityLabel="Developer" style={styles.iconButton} onPress={() => setDeveloperOpen(true)} hitSlop={8}>
            <Bug color={overlayIconColor} size={22} />
          </Pressable>
        ) : null}
        {rightActions}
        {photo && hasEdits ? (
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
      {photo ? <ImageInfoDialog photo={photo} visible={infoOpen} onClose={() => setInfoOpen(false)} /> : null}
      {__DEV__ ? <DeveloperDialog visible={developerOpen} onClose={() => setDeveloperOpen(false)} /> : null}
      <PresetDialog visible={presetsOpen} onClose={() => setPresetsOpen(false)} />
      {photo ? <ExportDialog
        items={exportItems}
        visible={saveOpen}
        onClose={() => setSaveOpen(false)}
        onSaveImage={saveCurrentPhoto}
      /> : null}
    </View>
  );
}

function createStyles(colors: ReturnType<typeof useTheme>['colors']) {
  return StyleSheet.create({
    topBar: {
      position: 'absolute',
      zIndex: 10,
      elevation: 10,
      left: 12,
      right: 12,
      flexDirection: 'row',
      alignItems: 'center',
      justifyContent: 'space-between',
    },
    titleWrap: { position: 'absolute', left: 48, right: 48, alignItems: 'center' },
    title: { color: '#fff', fontSize: 16, fontWeight: '600' },
    rightScroll: { flexGrow: 0, flexShrink: 1, marginLeft: 12 },
    right: {
      flexDirection: 'row',
      alignItems: 'center',
      gap: 12,
    },
    disabled: { opacity: 0.5 },
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
