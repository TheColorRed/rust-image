import { useEffect, useMemo, useState } from 'react';
import { Modal, Pressable, StyleSheet, Text, View } from 'react-native';
import { useObservable } from 'react-rx';
import { detectSkin } from '@/src/lib/ai/analysis';
import { downloadModel, refreshAiModels, syncAiModels } from '@/src/lib/ai/ai-model-catalog';
import { errorMessage } from '@/src/lib/utils/error-message';
import { formatBytes } from '@/src/lib/utils/format-bytes';
import { useTheme } from '@/src/lib/ui/theme';
import {
  aiModelList,
  aiModelsOfferDeclined,
  aiModelsRequested,
  aiModelsRequested$,
  aiModelsVersion,
  type AiModelListing,
} from '@/src/state/ai-models';
import { isSkinControlFocused$ } from '@/src/state/edits';
import { currentImage, editBaseImage, originalImage } from '@/src/state/session';

/**
 * Offers to download AI models in two cases:
 * - A skin control opens and the models are not on the device yet. Skin tools work without them (using color-based skin
 *   detection); the models only make them more precise, so this is an optional upgrade.
 * - An action that needs models is tapped without them (`aiModelsRequested`). Accepting downloads them and then applies the
 *   action; cancelling drops the request and downloads nothing.
 */
export function AiModelPrompt() {
  const { colors } = useTheme();
  const skinControlFocused = useObservable(isSkinControlFocused$, false);
  const request = useObservable(aiModelsRequested$, null);
  const [missing, setMissing] = useState<AiModelListing[]>([]);
  const [downloading, setDownloading] = useState(false);
  /** How far the download has got, from 0 to 1 across every model being downloaded. */
  const [progress, setProgress] = useState(0);
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
    if (request || !skinControlFocused || aiModelsOfferDeclined.value) {
      setMissing([]);
      return;
    }
    // The model list comes from S3; when it can't be reached there is nothing to offer.
    let cancelled = false;
    refreshAiModels().then(
      models => !cancelled && setMissing(models.filter(model => model.enabled && !model.downloaded)),
      () => {},
    );
    return () => {
      cancelled = true;
    };
  }, [skinControlFocused, request]);

  // An action that needs models: find which of them are missing, and say so when the list does not have them.
  useEffect(() => {
    if (!request) return;
    setError(null);
    let cancelled = false;
    refreshAiModels().then(
      models => {
        if (cancelled) return;
        const needed = models.filter(model => request.roles.includes(model.id));
        const unlisted = request.roles.filter(role => !needed.some(model => model.id === role));
        const notDownloaded = needed.filter(model => !model.downloaded);
        if (unlisted.length > 0) setError(`These AI models are not available yet: ${unlisted.join(', ')}.`);
        else if (notDownloaded.length === 0) setError('The AI models are on this device but could not be loaded.');
        setMissing(notDownloaded);
      },
      e => !cancelled && setError(errorMessage(e)),
    );
    return () => {
      cancelled = true;
    };
  }, [request]);

  const download = async () => {
    setError(null);
    setProgress(0);
    setDownloading(true);
    try {
      for (const [index, model] of missing.entries()) {
        await downloadModel(model, { update: share => setProgress((index + share) / missing.length) });
      }
      syncAiModels();
      if (request) {
        // Now that the models are here, do what was asked.
        setMissing([]);
        aiModelsRequested.next(null);
        aiModelsVersion.next(aiModelsVersion.value + 1);
        request.onReady();
        setDownloading(false);
        return;
      }
      // Switch the open photo from color-based to AI skin detection.
      const photo = originalImage.value ?? editBaseImage.value ?? currentImage.value;
      if (photo) {
        await detectSkin(photo);
      }
      setMissing([]);
      aiModelsVersion.next(aiModelsVersion.value + 1);
    } catch (e) {
      setError(errorMessage(e));
      syncAiModels();
      setMissing(
        aiModelList.value.filter(
          model => !model.downloaded && (request ? request.roles.includes(model.id) : model.enabled),
        ),
      );
    }
    setDownloading(false);
  };

  const decline = () => {
    if (request) {
      aiModelsRequested.next(null);
    } else {
      aiModelsOfferDeclined.next(true);
    }
    setMissing([]);
  };

  const totalBytes = missing.reduce((sum, model) => sum + model.downloadSizeBytes, 0);

  return (
    <Modal visible={request !== null || missing.length > 0} transparent animationType="fade" onRequestClose={decline}>
      <View style={styles.backdrop}>
        <View style={styles.dialog}>
          <Text style={styles.title}>{request ? 'Download AI models?' : 'Improve body detection?'}</Text>
          <Text style={styles.text}>
            {request
              ? missing.length > 0
                ? `${request.title} needs ${missing.map(model => model.title).join(' and ')} (${formatBytes(totalBytes)}). Download it once and keep it on this device.`
                : `${request.title} needs AI models.`
              : `Skin tools already work, but AI models find bodies more precisely. Download ${missing
                  .map(model => model.title)
                  .join(' and ')} (${formatBytes(totalBytes)}) once and keep it on this device.`}
          </Text>
          {error && <Text style={styles.error}>{error}</Text>}
          <View style={styles.buttons}>
            <Pressable
              accessibilityRole="button"
              disabled={downloading}
              onPress={decline}
              style={[styles.button, styles.decline]}
            >
              <Text style={styles.declineText}>{request ? 'Cancel' : 'Not now'}</Text>
            </Pressable>
            <Pressable
              accessibilityRole="button"
              disabled={downloading || (request !== null && missing.length === 0)}
              onPress={download}
              style={[styles.button, styles.accept]}
            >
              <Text style={styles.acceptText}>{downloading ? `${Math.round(progress * 100)}%` : 'Download'}</Text>
            </Pressable>
          </View>
        </View>
      </View>
    </Modal>
  );
}
