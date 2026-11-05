import { AiModelPrompt } from '@/src/components/editor/ai-model-prompt';
import { AiPendingNotice } from '@/src/components/editor/ai-pending-notice';
import { AiProgressBar } from '@/src/components/editor/ai-progress-bar';
import { EditorPanel } from '@/src/components/editor/editor-panel';
import { EditorPreview } from '@/src/components/editor/editor-preview';
import { EditorToolbar } from '@/src/components/editor/editor-toolbar';
import { type EditablePhoto } from '@/src/lib/editor/edit-source';
import { openEditorSession } from '@/src/lib/editor/editor-session';
import { editorSectionScrollOffset, setEditorSectionScrollOffset } from '@/src/lib/editor/editor-session';
import { clearScrollOffsets } from '@/src/lib/editor/scroll-offsets';
import { useTheme } from '@/src/lib/ui/theme';
import { closeRequested } from '@/src/state/commands';
import { aiProgress, aiProgress$ } from '@/src/state/ai-progress';
import { useEffect } from 'react';
import { BackHandler, View } from 'react-native';
import { useObservable } from 'react-rx';

export type { EditablePhoto } from '@/src/lib/editor/edit-source';

export interface EditScreenProps {
  photo: EditablePhoto;
  onClose: () => void;
}

/**
 * A Google Photos-style editor: a full-bleed preview on top, and a bottom panel with a row of
 * section tabs and a row of that section's controls. Follows the app's light/dark theme.
 *
 * All editor state lives in the rxjs controller (`events/editor`); this screen only opens the
 * photo session and lays out the children that subscribe to it.
 */
export function EditScreen({ photo, onClose }: EditScreenProps) {
  const { colors } = useTheme();
  const aiTasks = useObservable(aiProgress$, aiProgress.value);

  useEffect(() => openEditorSession(photo), [photo.uri]);

  // Back gesture / button does the same thing as the X button instead of closing the app.
  useEffect(() => {
    const subscription = BackHandler.addEventListener('hardwareBackPress', () => {
      closeRequested.next();
      return true;
    });
    return () => subscription.remove();
  }, []);

  return (
    <View style={{ flex: 1, backgroundColor: colors.background }}>
      <EditorPreview />
      <EditorToolbar
        photo={photo}
        onClose={() => {
          setEditorSectionScrollOffset(0);
          clearScrollOffsets();
          onClose();
        }}
      />
      <AiProgressBar tasks={aiTasks} />
      <EditorPanel
        sectionScrollOffset={editorSectionScrollOffset}
        onSectionScroll={setEditorSectionScrollOffset}
      />
      <AiPendingNotice />
      <AiModelPrompt />
    </View>
  );
}
