import { useState } from 'react';
import { Alert, Button, Linking, StyleSheet, Text, View } from 'react-native';
import { Camera as VisionCamera, useCameraPermission, usePhotoOutput } from 'react-native-vision-camera';

/**
 * Android camera using `react-native-vision-camera`.
 * - Requests camera permission, shows a live preview, supports front/back switching and capture.
 *
 * Props:
 * - onCapture(uri: string): called with the `file://` URI of the captured JPEG.
 * - initialFacing: 'user' | 'environment' (maps to front/back)
 */
export interface CameraProps {
  onCapture?: (uri: string) => void;
  initialFacing?: 'user' | 'environment';
  style?: any;
}

export default function Camera({ onCapture, initialFacing = 'environment', style }: CameraProps) {
  const { hasPermission, canRequestPermission, requestPermission } = useCameraPermission();
  const [cameraType, setCameraType] = useState<'back' | 'front'>(initialFacing === 'user' ? 'front' : 'back');
  const photoOutput = usePhotoOutput({});

  function switchCamera() {
    setCameraType((prev) => (prev === 'back' ? 'front' : 'back'));
  }

  async function takeSnapshot() {
    if (!hasPermission) {
      Alert.alert('Permission denied', 'Camera permission was denied. Please enable it in settings.');
      return;
    }
    try {
      const photo = await photoOutput.capturePhotoToFile({}, {});
      const uri = `file://${photo.filePath}`;
      onCapture?.(uri);
      return uri;
    } catch (e: any) {
      Alert.alert('Capture failed', String(e?.message ?? e));
    }
  }

  return (
    <View style={[styles.container, style]}>
      <View style={styles.preview}>
        {hasPermission ? (
          <VisionCamera style={styles.camera} isActive device={cameraType} outputs={[photoOutput]} />
        ) : (
          <View style={styles.centered}>
            <Text style={styles.small}>Camera permission is required.</Text>
            <Button
              title={canRequestPermission ? 'Request permission' : 'Open settings'}
              onPress={() => (canRequestPermission ? requestPermission() : Linking.openSettings())}
            />
          </View>
        )}
      </View>

      <View style={styles.controls}>
        <Button title="Capture" onPress={takeSnapshot} />
        <Button title="Switch Camera" onPress={switchCamera} />
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    width: '100%',
    height: 360,
    backgroundColor: '#000',
  },
  preview: {
    flex: 1,
    backgroundColor: '#000',
    overflow: 'hidden',
    justifyContent: 'center',
  },
  camera: { flex: 1 },
  controls: {
    paddingVertical: 8,
    flexDirection: 'row',
    justifyContent: 'space-around',
  },
  centered: {
    ...StyleSheet.absoluteFillObject,
    alignItems: 'center',
    justifyContent: 'center',
    gap: 8,
  },
  small: {
    color: '#fff',
  },
});
