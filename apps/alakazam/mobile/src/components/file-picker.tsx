import { useTheme } from '@/src/lib/ui/theme';
import { errorCodes, isErrorWithCode, keepLocalCopy, pick, types } from '@react-native-documents/picker';
import ImagePlus from 'lucide-react-native/icons/image-plus';
import { useMemo, useRef, useState } from 'react';
import { Alert, Button, Image, Platform, Pressable, StyleSheet, Text, View } from 'react-native';
import { Card, CardHeader } from './card';
import { ImagePreview } from './image-preview';
import { ProjectImagePicker } from './project-image-picker';
import { SelectableThumb, useSelection } from './selection';

export interface PickedImage {
  uri: string;
  name?: string;
  size?: number | null;
  mimeType?: string | null;
  fileCopyUri?: string | null;
}

export interface FilePickerProps {
  /** Called once with every image that was chosen and copied, in the order they were chosen. */
  onPick?: (assets: PickedImage[]) => void;
  title?: string;
  subtitle?: string;
  variant?: 'default' | 'card' | 'inline';
  /** For the inline variant: leave out the title, when the caller shows the name beside the control. The button then fills the width. */
  hideTitle?: boolean;
  showPreview?: boolean;
  multiple?: boolean;
  /** Where to pick from: the device's files, the saved projects (their edited images), or both. Defaults to `'file-system'`. */
  source?: 'file-system' | 'projects' | 'both';
  /** A project to leave out of the project grid, such as the one being edited. */
  excludeProjectId?: string;
  /** Images to show as thumbnails below the control, for a caller that keeps its own picked list. */
  thumbs?: PickedImage[];
  /** Lets the thumbnails be selected (press and hold one). The caller owns it so it can show the `SelectionBar` at the bottom of the screen. */
  selection?: ReturnType<typeof useSelection>;
  style?: any;
}

/**
 * Android-native image picker using the system document picker.
 * Uses the platform file chooser to select an existing image and returns its metadata + URI.
 */
export function FilePicker({
  onPick,
  title = 'Pick image from device',
  subtitle = 'Browse this device or a connected photo app',
  variant = 'default',
  hideTitle = false,
  showPreview = true,
  multiple = true,
  source = 'file-system',
  excludeProjectId,
  thumbs,
  selection,
  style,
}: FilePickerProps) {
  const { colors } = useTheme();
  const styles = useMemo(
    () =>
      StyleSheet.create({
        container: { width: '100%', gap: 8 },
        row: { flexDirection: 'row', alignItems: 'center', gap: 8 },
        inlineRow: { width: '100%', minHeight: 44, flexDirection: 'row', alignItems: 'center', justifyContent: 'space-between', gap: 12 },
        inlineButton: { minHeight: 36, minWidth: 88, paddingHorizontal: 14, borderRadius: 8, alignItems: 'center', justifyContent: 'center', backgroundColor: colors.accent },
        inlineButtonText: { color: '#fff', fontSize: 14, fontWeight: '600' },
        badge: { color: colors.success, fontSize: 12 },
        error: { color: colors.danger },
        preview: { gap: 6 },
        label: { fontWeight: '600', color: colors.textPrimary },
        thumbRow: { width: '100%', flexDirection: 'row', flexWrap: 'wrap', gap: 6 },
        thumb: { width: 56, height: 56, borderRadius: 6, backgroundColor: colors.placeholder },
        image: { width: '100%', height: 220, borderRadius: 12, backgroundColor: colors.placeholder },
        meta: { gap: 2 },
        metaText: { fontSize: 12, color: colors.textSecondary },
      }),
    [colors],
  );
  const picking = useRef(false);
  const unusedSelection = useSelection();
  const [previewIndex, setPreviewIndex] = useState<number | null>(null);
  const [selected, setSelected] = useState<PickedImage | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [projectsOpen, setProjectsOpen] = useState(false);

  /** The button: the device's chooser, or the project grid (which can hand over to the chooser when the source is both). */
  const openPicker = source === 'file-system' ? openDevicePicker : () => setProjectsOpen(true);

  async function openDevicePicker() {
    if (picking.current) return;
    picking.current = true;
    try {
      const [first, ...rest] = await pick({ type: [types.images], allowMultiSelection: multiple });
      const files = [first, ...rest];

      // Android returns content:// URIs; copy them into the app cache so they have real file paths. Results come back in order.
      const [firstCopy, ...restCopies] = await keepLocalCopy({
        files: [
          { uri: first.uri, fileName: first.name ?? 'image' },
          ...rest.map(file => ({ uri: file.uri, fileName: file.name ?? 'image' })),
        ],
        destination: 'cachesDirectory',
      });
      const failures: string[] = [];
      const assets: PickedImage[] = [];
      [firstCopy, ...restCopies].forEach((copy, index) => {
        const file = files[index];
        if (copy.status !== 'success') {
          failures.push(`${file.name ?? 'image'}: ${copy.copyError}`);
          return;
        }
        const asset: PickedImage = {
          uri: copy.localUri,
          name: file.name ?? 'image',
          size: file.size,
          mimeType: file.type,
          fileCopyUri: copy.localUri,
        };
        assets.push(asset);
      });
      if (assets.length > 0) {
        setSelected(assets[assets.length - 1]);
        onPick?.(assets);
      }
      setError(failures.length > 0 ? `Could not copy ${failures.length} image(s):\n${failures.join('\n')}` : null);
    } catch (e: any) {
      if (isErrorWithCode(e) && (e.code === errorCodes.OPERATION_CANCELED || e.code === errorCodes.IN_PROGRESS)) {
        return;
      }
      setError(String(e?.message ?? e));
      Alert.alert('Pick failed', String(e?.message ?? e));
    } finally {
      picking.current = false;
    }
  }

  return (
    <View style={[styles.container, style]}>
      {variant === 'card' ? (
        <Card title={title} subtitle={subtitle} onPress={openPicker}>
          <CardHeader icon={ImagePlus} title={title} subtitle={subtitle} />
        </Card>
      ) : variant === 'inline' ? (
        <View style={styles.inlineRow}>
          {hideTitle ? null : <Text numberOfLines={1} style={[styles.label, { flex: 1 }]}>{title}</Text>}
          <Pressable accessibilityRole="button" accessibilityLabel={`${title}: choose image`} style={[styles.inlineButton, hideTitle && { flex: 1 }]} onPress={openPicker}>
            <Text style={styles.inlineButtonText}>Choose</Text>
          </Pressable>
        </View>
      ) : (
        <View style={styles.row}>
          <Button title={title} onPress={openPicker} />
          {Platform.OS === 'android' && <Text style={styles.badge}>Android native picker</Text>}
        </View>
      )}

      {error && <Text style={styles.error}>{error}</Text>}

      {source === 'file-system' ? null : (
        <ProjectImagePicker
          visible={projectsOpen}
          multiple={multiple}
          excludeId={excludeProjectId}
          onBrowseDevice={source === 'both' ? openDevicePicker : undefined}
          onPick={assets => {
            setSelected(assets[assets.length - 1]);
            onPick?.(assets);
          }}
          onClose={() => setProjectsOpen(false)}
        />
      )}

      {thumbs && thumbs.length > 0 && (
        <View style={styles.thumbRow}>
          {thumbs.map((file, index) => (
            <SelectableThumb
              key={`${file.uri}-${index}`}
              id={file.uri}
              selection={selection ?? unusedSelection}
              selectable={selection !== undefined}
              onOpen={() => setPreviewIndex(index)}
              accessibilityLabel={`Open ${file.name ?? `image ${index + 1}`}`}
            >
              <Image source={{ uri: file.uri }} style={styles.thumb} resizeMode="cover" />
            </SelectableThumb>
          ))}
        </View>
      )}

      {thumbs && (
        <ImagePreview
          images={thumbs.map((file, index) => ({ id: `${file.uri}-${index}`, uri: file.uri }))}
          visible={previewIndex != null}
          initialIndex={previewIndex ?? 0}
          onClose={() => setPreviewIndex(null)}
        />
      )}

      {showPreview && selected && (
        <View style={styles.preview}>
          <Text style={styles.label}>Selected image</Text>
          <Image source={{ uri: selected.uri }} style={styles.image} resizeMode="cover" />
          <View style={styles.meta}>
            {selected.name && <Text style={styles.metaText}>Name: {selected.name}</Text>}
            {selected.mimeType && <Text style={styles.metaText}>Type: {selected.mimeType}</Text>}
            {selected.size != null && <Text style={styles.metaText}>Size: {Math.round(selected.size / 1024)} KB</Text>}
            <Text style={styles.metaText}>URI: {selected.uri}</Text>
          </View>
        </View>
      )}
    </View>
  );
}
