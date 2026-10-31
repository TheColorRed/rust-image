import { projectImageUri, type Project } from '@/src/lib/project/project-store';
import { useTheme } from '@/src/lib/ui/theme';
import { projects$ } from '@/src/state/projects';
import { useMemo, useState } from 'react';
import { keepLocalCopy } from '@react-native-documents/picker';
import { Alert, FlatList, Image, Modal, Pressable, StyleSheet, Text, View } from 'react-native';
import { useObservable } from 'react-rx';
import type { PickedImage } from './file-picker';

type Props = {
  visible: boolean;
  multiple: boolean;
  /** A project to leave out of the grid, such as the one being edited, which cannot be picked into itself. */
  excludeId?: string;
  /** Shows a button that closes this and opens the device's file chooser instead. */
  onBrowseDevice?: () => void;
  onPick: (images: PickedImage[]) => void;
  onClose: () => void;
};

/** A grid of the saved projects. A picked project gives a copy of its edited image, or of its original if it was never edited. */
export function ProjectImagePicker({ visible, multiple, excludeId, onBrowseDevice, onPick, onClose }: Props) {
  const { colors } = useTheme();
  const styles = useMemo(() => createStyles(colors), [colors]);
  const all = useObservable(projects$, []);
  const projects = useMemo(() => all.filter(project => project.id !== excludeId), [all, excludeId]);
  const [chosen, setChosen] = useState<string[]>([]);

  /** Hands over a frozen copy of each project's image, so editing the project afterwards never changes what was picked. */
  const finish = async (picked: Project[]) => {
    setChosen([]);
    onClose();
    if (picked.length === 0) return;
    try {
      const copies = await keepLocalCopy({
        files: picked.map(project => ({ uri: projectImageUri(project), fileName: project.resultFile ?? project.sourceFile })) as [{ uri: string; fileName: string }, ...{ uri: string; fileName: string }[]],
        destination: 'cachesDirectory',
      });
      const images: PickedImage[] = [];
      copies.forEach((copy, index) => {
        if (copy.status === 'success') images.push({ uri: copy.localUri, name: picked[index].name, fileCopyUri: copy.localUri });
      });
      if (images.length > 0) onPick(images);
      if (images.length < copies.length) Alert.alert('Could not copy a project', `${copies.length - images.length} of ${copies.length} images could not be copied.`);
    } catch (error) {
      Alert.alert('Could not copy a project', error instanceof Error ? error.message : String(error));
    }
  };

  return (
    <Modal visible={visible} transparent animationType="fade" onRequestClose={onClose}>
      <View style={styles.backdrop}>
        <View style={styles.dialog}>
          <View style={styles.header}>
            <Text style={styles.title}>Projects</Text>
            <Pressable accessibilityLabel="Cancel choosing a project" onPress={() => { setChosen([]); onClose(); }}>
              <Text style={styles.cancel}>Cancel</Text>
            </Pressable>
          </View>
          {onBrowseDevice ? (
            <Pressable accessibilityRole="button" style={styles.browse} onPress={() => { setChosen([]); onClose(); onBrowseDevice(); }}>
              <Text style={styles.browseText}>Browse files on this device</Text>
            </Pressable>
          ) : null}
          <FlatList
            style={styles.grid}
            data={projects}
            numColumns={3}
            keyExtractor={project => project.id}
            columnWrapperStyle={styles.gridRow}
            ListEmptyComponent={<Text style={styles.empty}>No projects yet.</Text>}
            renderItem={({ item }) => {
              const index = chosen.indexOf(item.id);
              return (
                <Pressable
                  accessibilityRole="button"
                  accessibilityLabel={item.name}
                  accessibilityState={{ selected: index >= 0 }}
                  style={[styles.cell, index >= 0 && styles.cellChosen]}
                  onPress={() => {
                    if (!multiple) return finish([item]);
                    setChosen(old => (old.includes(item.id) ? old.filter(id => id !== item.id) : [...old, item.id]));
                  }}
                >
                  <Image source={{ uri: projectImageUri(item) }} style={styles.thumb} resizeMode="cover" />
                  {index >= 0 ? <Text style={styles.badge}>{index + 1}</Text> : null}
                </Pressable>
              );
            }}
          />
          {multiple ? (
            <Pressable
              disabled={chosen.length === 0}
              style={[styles.primary, chosen.length === 0 && styles.disabled]}
              onPress={() => finish(chosen.flatMap(id => projects.filter(project => project.id === id)))}
            >
              <Text style={styles.primaryText}>{chosen.length === 0 ? 'Choose projects' : `Add ${chosen.length}`}</Text>
            </Pressable>
          ) : null}
        </View>
      </View>
    </Modal>
  );
}

function createStyles(c: ReturnType<typeof useTheme>['colors']) {
  return StyleSheet.create({
    backdrop: { flex: 1, justifyContent: 'center', padding: 24, backgroundColor: 'rgba(0,0,0,.6)' },
    dialog: { width: '100%', maxWidth: 480, maxHeight: '78%', alignSelf: 'center', gap: 12, padding: 20, borderRadius: 16, backgroundColor: c.surface },
    header: { flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center' },
    title: { color: c.textPrimary, fontSize: 18, fontWeight: '700' },
    cancel: { color: c.textSecondary, fontWeight: '600' },
    browse: { padding: 12, borderRadius: 10, alignItems: 'center', backgroundColor: c.chip },
    browseText: { color: c.accent, fontWeight: '600' },
    grid: { flexGrow: 0 },
    gridRow: { gap: 8, marginBottom: 8 },
    cell: { flex: 1 / 3, aspectRatio: 1, borderRadius: 8, overflow: 'hidden', borderWidth: 2, borderColor: 'transparent', backgroundColor: c.placeholder },
    cellChosen: { borderColor: c.accent },
    thumb: { width: '100%', height: '100%' },
    badge: { position: 'absolute', top: 4, right: 4, minWidth: 20, paddingHorizontal: 5, borderRadius: 10, overflow: 'hidden', textAlign: 'center', color: '#fff', fontSize: 12, fontWeight: '700', backgroundColor: c.accent },
    empty: { color: c.textSecondary, textAlign: 'center', padding: 20 },
    primary: { alignItems: 'center', paddingVertical: 11, borderRadius: 10, backgroundColor: c.accent },
    primaryText: { color: '#fff', fontWeight: '600' },
    disabled: { opacity: 0.4 },
  });
}
