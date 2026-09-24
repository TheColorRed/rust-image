import { useEffect, useRef, useState } from 'react';
import { ActivityIndicator, Alert, Button, StyleSheet, Text, View } from 'react-native';

// Synchronously require `expo-camera` so Metro bundles it into the Android app binary.
const ExpoCameraModule: any | null = (() => {
  try {
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    return require('expo-camera');
  } catch (e) {
    return null;
  }
})();

/**
 * Android-only Camera component using `expo-camera`.
 * - Requests camera permissions, shows a live preview, supports front/back switching and capture.
 *
 * Props:
 * - onCapture(dataUrl: string): called with a data URL (base64) of the captured image or a file URI.
 * - initialFacing: 'user' | 'environment' (maps to front/back)
 */
export interface CameraProps {
  onCapture?: (dataUrl: string) => void;
  initialFacing?: 'user' | 'environment';
  style?: any;
}

export default function Camera({ onCapture, initialFacing = 'environment', style }: CameraProps) {
  const cameraRef = useRef<any>(null);
  const [hasPermission, setHasPermission] = useState<boolean | null>(null);
  const [cameraType, setCameraType] = useState<'back' | 'front'>(initialFacing === 'user' ? 'front' : 'back');
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    (async () => {
      if (!ExpoCameraModule) {
        setError(
          '`expo-camera` native module not found. Rebuild the Android app (e.g. `expo run:android`) after installing `expo-camera`.',
        );
        setHasPermission(false);
        return;
      }

      try {
        const requestFn =
          ExpoCameraModule.requestCameraPermissionsAsync ?? ExpoCameraModule.Camera?.requestCameraPermissionsAsync;
        if (!requestFn) {
          setError('`expo-camera` requestCameraPermissionsAsync is not available.');
          setHasPermission(false);
          return;
        }
        const res = await requestFn();
        const status = (res && (res.status ?? res)) as any;
        setHasPermission(status === 'granted');
        setCameraType(initialFacing === 'user' ? 'front' : 'back');
      } catch (e: any) {
        setError(String(e?.message ?? e));
        setHasPermission(false);
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function switchCamera() {
    setCameraType((prev: 'back' | 'front') => (prev === 'back' ? 'front' : 'back'));
  }

  async function takeSnapshot() {
    if (!ExpoCameraModule) {
      Alert.alert('Camera not available', '`expo-camera` is not available. Install and rebuild the Android app.');
      return;
    }

    if (hasPermission === false) {
      Alert.alert('Permission denied', 'Camera permission was denied. Please enable it in settings.');
      return;
    }

    if (!cameraRef.current?.takePictureAsync) {
      Alert.alert('Camera not ready', 'The camera preview is not ready yet. Please try again.');
      return;
    }

    try {
      const photo = await cameraRef.current.takePictureAsync({ base64: true, quality: 0.8 });
      if (photo?.base64) {
        const dataUrl = `data:image/jpeg;base64,${photo.base64}`;
        onCapture?.(dataUrl);
        return dataUrl;
      }
      if (photo?.uri) {
        onCapture?.(photo.uri);
        return photo.uri;
      }
    } catch (e: any) {
      Alert.alert('Capture failed', String(e?.message ?? e));
    }
  }

  // Native UI using expo-camera (Android-only)
  const CameraComp = ExpoCameraModule?.CameraView ?? ExpoCameraModule?.default ?? null;

  return (
    <View style={[styles.container, style]}>
      <View style={styles.preview}>
        {hasPermission === null && (
          <View style={styles.centered}>
            <ActivityIndicator />
            <Text style={styles.small}>Requesting camera permission…</Text>
          </View>
        )}

        {error && (
          <View style={styles.centered}>
            <Text style={styles.error}>{error}</Text>
          </View>
        )}

        {hasPermission === false && !error && (
          <View style={styles.centered}>
            <Text style={styles.small}>Camera permission denied.</Text>
            <Button
              title="Request permission"
              onPress={async () => {
                if (!ExpoCameraModule) {
                  Alert.alert(
                    'Camera not available',
                    '`expo-camera` is not available. Install and rebuild the Android app.',
                  );
                  return;
                }
                try {
                  const requestFn =
                    ExpoCameraModule.requestCameraPermissionsAsync ??
                    ExpoCameraModule.Camera?.requestCameraPermissionsAsync;
                  if (!requestFn) {
                    Alert.alert('Permission error', 'requestCameraPermissionsAsync not available');
                    return;
                  }
                  const res = await requestFn();
                  const status = (res && (res.status ?? res)) as any;
                  setHasPermission(status === 'granted');
                } catch (e: any) {
                  Alert.alert('Permission error', String(e?.message ?? e));
                }
              }}
            />
          </View>
        )}

        {hasPermission === true && CameraComp && (
          // @ts-ignore - CameraView component
          <CameraComp ref={cameraRef} style={{ flex: 1 }} facing={cameraType} />
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

  controls: {
    paddingVertical: 8,
    flexDirection: 'row',
    justifyContent: 'space-around',
  } as any,
  centered: {
    position: 'absolute',
    left: 0,
    right: 0,
    top: 0,
    bottom: 0,
    alignItems: 'center',
    justifyContent: 'center',
  } as any,
  small: {
    color: '#fff',
    marginTop: 8,
  },
  error: {
    color: '#f88',
  },
});
