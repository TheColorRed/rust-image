import { aiModels, downloadAiModel, refreshAiModels, type AiModel } from '@alakazam/mobile';
import { useEffect, useMemo, useState } from 'react';
import { ActivityIndicator, Modal, Pressable, StyleSheet, Text, View } from 'react-native';
import { useObservable } from 'react-rx';
import { errorMessage } from '@/src/lib/error-message';
import { formatBytes } from '@/src/lib/format-bytes';
import { useTheme } from '@/src/lib/theme';
import { aiModelsOfferDeclined, aiModelsVersion } from '@/src/state/ai-models';
import { isSkinControlFocused$ } from '@/src/state/edits';
import { currentImage, editBaseImage } from '@/src/state/session';

/**
 * Offers to download the AI models when a skin control opens and they are not on the device yet. Skin tools work
 * without them (using color-based skin detection); the models only make them more precise.
 */
export function AiModelPrompt() {
  const { colors } = useTheme();
  const skinControlFocused = useObservable(isSkinControlFocused$, false);
  const [missing, setMissing] = useState<AiModel[]>([]);
  const [downloading, setDownloading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const styles = useMemo(
    () =>
      StyleSheet.create({
        backdrop: { flex: 1, justifyContent: 'center', padding: 24, backgroundColor: 'rgba(0,0,0,0.6)' },
        dialog: { gap: 12, padding: 20, borderRadius: 16, backgroundColor: colors.surface },
        title: { fontSize: 18, fontWeight: '600', color: colors.textPrimary },
        text: { fontSize: 14, color: colors.textSecondary },
        error: { fontSize: 13, color: colors.danger },
        buttons: { flexDirection: 'row', justifyContent: 'flex-end', gap: 12, marginTop: 4 },
        button: { minWidth: 96, paddingHorizontal: 16, paddingVertical: 10, borderRadius: 10, alignItems: 'center' },
        decline: { borderWidth: 1, borderColor: colors.border },
        accept: { backgroundColor: colors.accent },
        declineText: { fontSize: 15, fontWeight: '600', color: colors.textPrimary },
        acceptText: { fontSize: 15, fontWeight: '600', color: '#fff' },
      }),
    [colors],
  );

  useEffect(() => {
    setError(null);
    if (!skinControlFocused || aiModelsOfferDeclined.value) {
      setMissing([]);
      return;
    }
    // The model list comes from S3; when it can't be reached there is nothing to offer.
    let cancelled = false;
    refreshAiModels().then(
      models => !cancelled && setMissing(models.filter(model => !model.downloaded)),
      () => {},
    );
    return () => {
      cancelled = true;
    };
  }, [skinControlFocused]);

  const download = async () => {
    setError(null);
    setDownloading(true);
    try {
      for (const model of missing) await downloadAiModel(model.id);
      // Switch the open photo from color-based to AI skin detection.
      const images = [currentImage.value, editBaseImage.value].filter(image => image !== null);
      await Promise.all(images.map(image => image.detectSkinAsync()));
      setMissing([]);
      aiModelsVersion.next(aiModelsVersion.value + 1);
    } catch (e) {
      setError(errorMessage(e));
      setMissing(aiModels().filter(model => !model.downloaded));
    }
    setDownloading(false);
  };

  const decline = () => {
    aiModelsOfferDeclined.next(true);
    setMissing([]);
  };

  const totalBytes = missing.reduce((sum, model) => sum + Number(model.downloadSizeBytes), 0);

  return (
    <Modal visible={missing.length > 0} transparent animationType="fade" onRequestClose={decline}>
      <View style={styles.backdrop}>
        <View style={styles.dialog}>
          <Text style={styles.title}>Improve body detection?</Text>
          <Text style={styles.text}>
            Skin tools already work, but AI models find bodies more precisely. Download{' '}
            {missing.map(model => model.title).join(' and ')} ({formatBytes(totalBytes)}) once and keep it on this device.
          </Text>
          {error && <Text style={styles.error}>{error}</Text>}
          <View style={styles.buttons}>
            <Pressable accessibilityRole="button" disabled={downloading} onPress={decline} style={[styles.button, styles.decline]}>
              <Text style={styles.declineText}>Not now</Text>
            </Pressable>
            <Pressable accessibilityRole="button" disabled={downloading} onPress={download} style={[styles.button, styles.accept]}>
              {downloading ? <ActivityIndicator color="#fff" /> : <Text style={styles.acceptText}>Download</Text>}
            </Pressable>
          </View>
        </View>
      </View>
    </Modal>
  );
}
