import * as DocumentPicker from 'expo-document-picker';
import { useState } from 'react';
import { Alert, Button, Image, Platform, StyleSheet, Text, View } from 'react-native';

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
  style?: any;
}

/**
 * Android-native image picker using Expo Document Picker.
 * Uses the platform file chooser to select an existing image and returns its metadata + URI.
 */
export default function FilePicker({ onPick, title = 'Pick image from device', style }: FilePickerProps) {
  const [selected, setSelected] = useState<PickedImage | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function openPicker() {
    try {
      const result = await DocumentPicker.getDocumentAsync({
        type: ['image/*'],
        copyToCacheDirectory: true,
        multiple: false,
      });

      // Canceled selection
      if ((result as any)?.canceled || (result as any)?.type === 'cancel') {
        return;
      }

      let asset: PickedImage | null = null;

      // New SDK shape: { assets: [{ uri, name, size, mimeType }] }
      if ((result as any)?.assets?.length) {
        const first = (result as any).assets[0];
        asset = {
          uri: first?.uri,
          name: first?.name,
          size: first?.size ?? null,
          mimeType: first?.mimeType ?? null,
          fileCopyUri: first?.fileCopyUri ?? null,
        };
      }

      // Legacy shape: { type: 'success', uri, name, size, mimeType }
      if (!asset && (result as any)?.type === 'success') {
        asset = {
          uri: (result as any).uri,
          name: (result as any).name,
          size: (result as any).size ?? null,
          mimeType: (result as any).mimeType ?? null,
          fileCopyUri: (result as any).fileCopyUri ?? null,
        };
      }

      if (!asset?.uri) {
        setError('No image returned from the picker.');
        return;
      }

      setSelected(asset);
      setError(null);
      onPick?.(asset);
    } catch (e: any) {
      setError(String(e?.message ?? e));
      Alert.alert('Pick failed', String(e?.message ?? e));
    }
  }

  return (
    <View style={[styles.container, style]}>
      <View style={styles.row}>
        <Button title={title} onPress={openPicker} />
        {Platform.OS === 'android' && <Text style={styles.badge}>Android native picker</Text>}
      </View>

      {error && <Text style={styles.error}>{error}</Text>}

      {selected && (
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

const styles = StyleSheet.create({
  container: { width: '100%', padding: 16, gap: 8 },
  row: { flexDirection: 'row', alignItems: 'center', gap: 8 },
  badge: { color: '#4a7', fontSize: 12 },
  error: { color: '#d22' },
  preview: { gap: 6 },
  label: { fontWeight: '600' },
  image: { width: '100%', height: 220, borderRadius: 12, backgroundColor: '#111' },
  meta: { gap: 2 },
  metaText: { fontSize: 12, color: '#444' },
});
