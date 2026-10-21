import { useMemo } from 'react';
import { Alert, Modal, Pressable, StyleSheet, Text, View } from 'react-native';
import { resetProjectToNew } from '@/src/lib/editor/editor-session';
import { errorMessage } from '@/src/lib/utils/error-message';
import { useTheme } from '@/src/lib/ui/theme';

export interface DeveloperDialogProps {
  visible: boolean;
  onClose: () => void;
}

/** A dialog of actions for developers, shown only in development builds. */
export function DeveloperDialog({ visible, onClose }: DeveloperDialogProps) {
  const { colors } = useTheme();
  const styles = useMemo(
    () =>
      StyleSheet.create({
        backdrop: { flex: 1, justifyContent: 'center', alignItems: 'center', padding: 24, backgroundColor: 'rgba(0,0,0,0.6)' },
        dialog: { width: '100%', maxWidth: 480, gap: 12, padding: 20, borderRadius: 16, backgroundColor: colors.surface },
        title: { fontSize: 18, fontWeight: '600', color: colors.textPrimary },
        text: { fontSize: 13, color: colors.textSecondary },
        action: { paddingHorizontal: 16, paddingVertical: 12, borderRadius: 10, alignItems: 'center', backgroundColor: colors.danger },
        actionText: { fontSize: 15, fontWeight: '600', color: '#fff' },
        buttons: { flexDirection: 'row', justifyContent: 'flex-end', marginTop: 4 },
        button: { minWidth: 96, paddingHorizontal: 16, paddingVertical: 10, borderRadius: 10, alignItems: 'center', backgroundColor: colors.accent },
        buttonText: { fontSize: 15, fontWeight: '600', color: '#fff' },
      }),
    [colors],
  );

  return (
    <Modal visible={visible} transparent animationType="fade" onRequestClose={onClose}>
      <View style={styles.backdrop}>
        <View style={styles.dialog}>
          <Text style={styles.title}>Developer</Text>
          <Text style={styles.text}>
            Takes every edit, the final image, the masks, the people and the face boxes off this project and closes the editor. Opening
            it again is like opening the photo for the first time.
          </Text>
          <Pressable
            accessibilityRole="button"
            style={styles.action}
            onPress={() =>
              Alert.alert('Reset the whole project?', 'This cannot be undone.', [
                { text: 'Cancel', style: 'cancel' },
                {
                  text: 'Reset',
                  style: 'destructive',
                  onPress: () => {
                    onClose();
                    resetProjectToNew().catch(error => Alert.alert('Could not reset the project', errorMessage(error)));
                  },
                },
              ])
            }
          >
            <Text style={styles.actionText}>Reset project to new</Text>
          </Pressable>
          <View style={styles.buttons}>
            <Pressable accessibilityRole="button" onPress={onClose} style={styles.button}>
              <Text style={styles.buttonText}>Close</Text>
            </Pressable>
          </View>
        </View>
      </View>
    </Modal>
  );
}
