import { useTheme } from '@/src/lib/theme';
import { errorCodes, isErrorWithCode, keepLocalCopy, pick, types } from '@react-native-documents/picker';
import { ImagePlus } from 'lucide-react-native';
import { useMemo, useState } from 'react';
import { Alert, Button, Image, Platform, StyleSheet, Text, View } from 'react-native';
import { Card, CardHeader } from './card';

export interface PickedImage {
  uri: string;
  name?: string;
  size?: number | null;
  mimeType?: string | null;
  fileCopyUri?: string | null;
}

export interface FilePickerProps {
  onPick?: (asset: PickedImage) => void;
  title?: string;
  subtitle?: string;
  variant?: 'default' | 'card';
  showPreview?: boolean;
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
  showPreview = true,
  style,
}: FilePickerProps) {
  const { colors } = useTheme();
  const styles = useMemo(
    () =>
      StyleSheet.create({
        container: { width: '100%', gap: 8 },
        row: { flexDirection: 'row', alignItems: 'center', gap: 8 },
        badge: { color: colors.success, fontSize: 12 },
        error: { color: colors.danger },
        preview: { gap: 6 },
        label: { fontWeight: '600', color: colors.textPrimary },
        image: { width: '100%', height: 220, borderRadius: 12, backgroundColor: colors.placeholder },
        meta: { gap: 2 },
        metaText: { fontSize: 12, color: colors.textSecondary },
      }),
    [colors],
  );
  const [selected, setSelected] = useState<PickedImage | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function openPicker() {
    try {
      const [first, ...rest] = await pick({ type: [types.images], allowMultiSelection: true });
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
        setSelected(asset);
        onPick?.(asset);
      });
      setError(failures.length > 0 ? `Could not copy ${failures.length} image(s):\n${failures.join('\n')}` : null);
    } catch (e: any) {
      if (isErrorWithCode(e) && e.code === errorCodes.OPERATION_CANCELED) {
        return;
      }
      setError(String(e?.message ?? e));
      Alert.alert('Pick failed', String(e?.message ?? e));
    }
  }

  return (
    <View style={[styles.container, style]}>
      {variant === 'card' ? (
        <Card title={title} subtitle={subtitle} onPress={openPicker}>
          <CardHeader icon={ImagePlus} title={title} subtitle={subtitle} />
        </Card>
      ) : (
        <View style={styles.row}>
          <Button title={title} onPress={openPicker} />
          {Platform.OS === 'android' && <Text style={styles.badge}>Android native picker</Text>}
        </View>
      )}

      {error && <Text style={styles.error}>{error}</Text>}

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
