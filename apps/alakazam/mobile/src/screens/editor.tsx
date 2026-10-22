import { AiModelPrompt } from '@/src/components/editor/ai-model-prompt';
import { AiPendingNotice } from '@/src/components/editor/ai-pending-notice';
import { AiProgressBar } from '@/src/components/editor/ai-progress-bar';
import { EditorPanel } from '@/src/components/editor/editor-panel';
import { EditorPreview } from '@/src/components/editor/editor-preview';
import { EditorToolbar } from '@/src/components/editor/editor-toolbar';
import { type EditablePhoto } from '@/src/lib/editor/edit-source';
import { openEditorSession } from '@/src/lib/editor/editor-session';
import { useTheme } from '@/src/lib/ui/theme';
import { closeRequested } from '@/src/state/commands';
import { bottomInset, topInset } from '@/src/state/preview';
import { aiProgress, aiProgress$ } from '@/src/state/ai-progress';
import { useEffect, useRef } from 'react';
import { BackHandler, View } from 'react-native';
import { useObservable } from 'react-rx';
import { useSafeAreaInsets } from 'react-native-safe-area-context';

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
 * photo session, feeds it the safe-area insets, and lays out the children that subscribe to it.
 */
export function EditScreen({ photo, onClose }: EditScreenProps) {
  const insets = useSafeAreaInsets();
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

  // The bottom inset (the system's gesture area) can be reported as 0 and as its real size by turns while the phone is busy, and
  // the panel pads itself by it, so the preview above it changed size each time and was drawn again at each size. The editor keeps
  // the largest it has seen while it is open: the panel may keep a little extra room if the area really goes away.
  const largestBottom = useRef(0);
  useEffect(() => {
    largestBottom.current = Math.max(largestBottom.current, insets.bottom);
    topInset.next(insets.top);
    bottomInset.next(largestBottom.current);
  }, [insets.top, insets.bottom]);

  return (
    <View style={{ flex: 1, backgroundColor: colors.background }}>
      <EditorPreview />
      <EditorToolbar photo={photo} onClose={onClose} />
      <AiProgressBar tasks={aiTasks} />
      <EditorPanel />
      <AiPendingNotice />
      <AiModelPrompt />
    </View>
  );
}
