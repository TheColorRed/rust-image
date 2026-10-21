import { formatBytes } from '@/src/lib/utils/format-bytes';
import { useTheme } from '@/src/lib/ui/theme';
import { people$, people as peopleState } from '@/src/state/analysis';
import { timeline$ } from '@/src/state/history';
import { previewSize$ } from '@/src/state/preview';
import { type EditablePhoto } from '@/src/lib/editor/edit-source';
import { editBaseImage$, originalImage$ } from '@/src/state/session';
import Clipboard from '@react-native-clipboard/clipboard';
import { useEffect, useMemo, useState } from 'react';
import { Modal, Pressable, StyleSheet, Text, View } from 'react-native';
import { useObservable } from 'react-rx';

export interface ImageInfoDialogProps {
  /** The photo being edited, which knows its file. */
  photo: EditablePhoto;
  visible: boolean;
  onClose: () => void;
}

const greatestCommonDivisor = (a: number, b: number): number => (b === 0 ? a : greatestCommonDivisor(b, a % b));

/** A dialog with what is known about the open photo: its file, its size and shape, and how it is drawn right now. */
export function ImageInfoDialog({ photo, visible, onClose }: ImageInfoDialogProps) {
  const { colors } = useTheme();
  const original = useObservable(originalImage$, null);
  const editBase = useObservable(editBaseImage$, null);
  const drawn = useObservable(previewSize$, null);
  const edits = useObservable(timeline$, { entries: [], cursor: 0 }).cursor;
  const found = useObservable(people$, peopleState.value);
  const [copied, setCopied] = useState(false);
  // The notice is for the last tap, so it is gone when the dialog is opened again.
  useEffect(() => setCopied(false), [visible]);
  const styles = useMemo(
    () =>
      StyleSheet.create({
        backdrop: {
          flex: 1,
          justifyContent: 'center',
          alignItems: 'center',
          padding: 24,
          backgroundColor: 'rgba(0,0,0,0.6)',
        },
        // The width of a phone at most, so on a tablet or a landscape screen the dialog does not stretch across it.
        dialog: {
          width: '100%',
          maxWidth: 480,
          gap: 12,
          padding: 20,
          borderRadius: 16,
          backgroundColor: colors.surface,
        },
        title: { fontSize: 18, fontWeight: '600', color: colors.textPrimary },
        row: { flexDirection: 'row', justifyContent: 'space-between', gap: 16 },
        label: { fontSize: 14, color: colors.textSecondary },
        value: { flexShrink: 1, fontSize: 14, fontWeight: '500', color: colors.textPrimary, textAlign: 'right' },
        buttons: { flexDirection: 'row', justifyContent: 'flex-end', marginTop: 4 },
        button: {
          minWidth: 96,
          paddingHorizontal: 16,
          paddingVertical: 10,
          borderRadius: 10,
          alignItems: 'center',
          backgroundColor: colors.accent,
        },
        buttonText: { fontSize: 15, fontWeight: '600', color: '#fff' },
      }),
    [colors],
  );

  const rows = useMemo(() => {
    if (!visible) return [];
    // A photo that was let go of by the session ending throws when asked its size, so each size is read with that in mind.
    const sizeOf = (image: { width(): number; height(): number } | null) => {
      try {
        return image ? { width: image.width(), height: image.height() } : null;
      } catch {
        return null;
      }
    };
    const size = sizeOf(original) ?? sizeOf(editBase);
    const current = sizeOf(editBase);
    const name = photo.fileName ?? photo.uri.split('/').pop()?.split('?')[0] ?? null;
    const extension = name?.includes('.') ? name.split('.').pop()!.toUpperCase() : null;
    const list: [string, string][] = [];
    if (name) list.push(['File', decodeURIComponent(name)]);
    if (extension) list.push(['Format', extension]);
    // The project's folder name on the phone, so a developer can find its files.
    if (__DEV__ && photo.project) list.push(['Project ID', photo.project.id]);
    if (size) {
      const divisor = greatestCommonDivisor(size.width, size.height);
      const shape = size.width === size.height ? 'Square' : size.width > size.height ? 'Landscape' : 'Portrait';
      list.push(['Size', `${size.width} × ${size.height} px`]);
      list.push(['Megapixels', `${((size.width * size.height) / 1_000_000).toFixed(1)} MP`]);
      list.push(['Shape', `${shape}, ${size.width / divisor}:${size.height / divisor}`]);
      list.push(['Memory in the editor', formatBytes(size.width * size.height * 4)]);
    }
    // After a crop or a rotation the photo being edited is no longer the one that was opened.
    if (size && current && (current.width !== size.width || current.height !== size.height)) {
      list.push(['Size now', `${current.width} × ${current.height} px`]);
    }
    // Null until the people have been looked for.
    list.push(['People', found === null ? 'Not found yet' : found.length === 0 ? 'None' : String(found.length)]);
    list.push(['Edits', edits === 0 ? 'None' : edits === 1 ? '1 edit' : `${edits} edits`]);
    if (drawn && current) {
      const full = drawn.width === current.width && drawn.height === current.height;
      list.push([
        'Drawn at',
        `${drawn.width} × ${drawn.height} px${full ? ' (full size)' : ' (scaled to the screen)'}`,
      ]);
    }
    return list;
  }, [visible, photo, original, editBase, drawn, edits, found]);

  return (
    <Modal visible={visible} transparent animationType="fade" onRequestClose={onClose}>
      <View style={styles.backdrop}>
        <View style={styles.dialog}>
          <Text style={styles.title}>Photo information</Text>
          {rows.length === 0 ? <Text style={styles.label}>Nothing is known about this photo yet.</Text> : null}
          {rows.map(([label, value]) => (
            <Pressable
              key={label}
              disabled={label !== 'Project ID'}
              onPress={() => {
                Clipboard.setString(value);
                setCopied(true);
              }}
              style={styles.row}
            >
              <Text style={styles.label}>{label}</Text>
              <Text style={styles.value}>{label === 'Project ID' && copied ? 'Copied to clipboard' : value}</Text>
            </Pressable>
          ))}
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
