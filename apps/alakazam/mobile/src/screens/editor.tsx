import { AiModelPrompt } from '@/src/components/editor/ai-model-prompt';
import { EditorPanel } from '@/src/components/editor/editor-panel';
import { EditorPreview } from '@/src/components/editor/editor-preview';
import { EditorToolbar } from '@/src/components/editor/editor-toolbar';
import { type EditablePhoto } from '@/src/lib/edit-source';
import { openEditorSession } from '@/src/lib/editor-session';
import { useTheme } from '@/src/lib/theme';
import { closeRequested } from '@/src/state/commands';
import { bottomInset, topInset } from '@/src/state/preview';
import { useEffect } from 'react';
import { BackHandler, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';

export type { EditablePhoto } from '@/src/lib/edit-source';

export interface EditScreenProps {
  photo: EditablePhoto;
  onClose: () => void;
}

/**
 * A Google Photos-style editor: a full-bleed preview on top, and a bottom panel with a row of
 * section tabs and a row of that section's controls. Follows the app's light/dark theme.
 *
 * All editor state lives in the rxjs controller (`events/editor`); this screen only opens the
 * photo session, feeds it the safe-area insets, and lays out the children that subscribe to it.
 */
export function EditScreen({ photo, onClose }: EditScreenProps) {
  const insets = useSafeAreaInsets();
  const { colors } = useTheme();

  useEffect(() => openEditorSession(photo), [photo.uri]);

  // Back gesture / button does the same thing as the X button instead of closing the app.
  useEffect(() => {
    const subscription = BackHandler.addEventListener('hardwareBackPress', () => {
      closeRequested.next();
      return true;
    });
    return () => subscription.remove();
  }, []);

  useEffect(() => {
    topInset.next(insets.top);
    bottomInset.next(insets.bottom);
  }, [insets.top, insets.bottom]);

  return (
    <View style={{ flex: 1, backgroundColor: colors.background }}>
      <EditorPreview />
      <EditorToolbar onClose={onClose} />
      <EditorPanel />
      <AiModelPrompt />
    </View>
  );
}
