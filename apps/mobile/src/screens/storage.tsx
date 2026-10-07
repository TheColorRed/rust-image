import { aiModels, deleteAiModel, downloadAiModel, refreshAiModels, type AiModel } from '@alakazam/mobile';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { ActivityIndicator, Pressable, ScrollView, StyleSheet, Text, View } from 'react-native';
import { AppText } from '@/src/components/app-text';
import { Card } from '@/src/components/card';
import { errorMessage } from '@/src/lib/error-message';
import { formatBytes } from '@/src/lib/format-bytes';
import { useCommonStyles } from '@/src/lib/styles';
import { useTheme } from '@/src/lib/theme';

export default function StorageScreen() {
  const { colors } = useTheme();
  const commonStyles = useCommonStyles();
  const [models, setModels] = useState<AiModel[]>([]);
  const [downloading, setDownloading] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const styles = useMemo(
    () =>
      StyleSheet.create({
        container: { flex: 1, backgroundColor: colors.background, paddingTop: 24 },
        content: { paddingHorizontal: 16, gap: 12 },
        row: {
          flexDirection: 'row',
          alignItems: 'center',
          justifyContent: 'space-between',
          gap: 12,
          paddingHorizontal: 16,
          paddingVertical: 12,
          borderTopWidth: 1,
          borderTopColor: colors.border,
        },
        rowText: { flex: 1 },
        rowTitle: { fontSize: 16, color: colors.textPrimary },
        rowSize: { marginTop: 2, fontSize: 13, color: colors.textSecondary },
        button: { minWidth: 96, paddingVertical: 8, borderRadius: 8, alignItems: 'center' },
        download: { backgroundColor: colors.accent },
        remove: { borderWidth: 1, borderColor: colors.danger },
        buttonText: { fontSize: 14, fontWeight: '600' },
        note: { fontSize: 13, color: colors.textSecondary },
        error: { fontSize: 13, color: colors.danger },
      }),
    [colors],
  );

  const refresh = useCallback(() => setModels(aiModels()), []);
  // The model list comes from S3, so it is fetched each time the tab opens.
  useEffect(() => {
    refreshAiModels().then(setModels, e => setError(errorMessage(e)));
  }, []);

  const download = async (model: AiModel) => {
    setError(null);
    setDownloading(model.id);
    try {
      await downloadAiModel(model.id);
    } catch (e) {
      setError(errorMessage(e));
    }
    setDownloading(null);
    refresh();
  };

  const remove = (model: AiModel) => {
    setError(null);
    try {
      deleteAiModel(model.id);
    } catch (e) {
      setError(errorMessage(e));
    }
    refresh();
  };

  return (
    <View style={styles.container}>
      <View style={commonStyles.header}>
        <AppText size="heading" bold>
          Storage
        </AppText>
      </View>

      <ScrollView contentContainerStyle={styles.content}>
        <Card title="AI models">
          {models.map((model) => (
            <View key={model.id} style={styles.row}>
              <View style={styles.rowText}>
                <Text style={styles.rowTitle}>{model.title}</Text>
                {model.description !== '' && <Text style={styles.rowSize}>{model.description}</Text>}
                <Text style={styles.rowSize}>
                  {model.downloaded
                    ? `Downloaded · ${formatBytes(Number(model.sizeBytes))}`
                    : `Not downloaded · ${formatBytes(Number(model.downloadSizeBytes))}`}
                </Text>
              </View>
              {downloading === model.id ? (
                <View style={styles.button}>
                  <ActivityIndicator color={colors.accent} />
                </View>
              ) : model.downloaded ? (
                <Pressable
                  accessibilityRole="button"
                  accessibilityLabel={`Delete ${model.title}`}
                  onPress={() => remove(model)}
                  style={[styles.button, styles.remove]}
                >
                  <Text style={[styles.buttonText, { color: colors.danger }]}>Delete</Text>
                </Pressable>
              ) : (
                <Pressable
                  accessibilityRole="button"
                  accessibilityLabel={`Download ${model.title}`}
                  disabled={downloading !== null}
                  onPress={() => download(model)}
                  style={[styles.button, styles.download, downloading !== null && { opacity: 0.4 }]}
                >
                  <Text style={[styles.buttonText, { color: '#fff' }]}>Download</Text>
                </Pressable>
              )}
            </View>
          ))}
        </Card>
        <Text style={styles.note}>
          AI features download their models the first time you use them. Delete one to free space; it downloads again
          when needed.
        </Text>
        {error && <Text style={styles.error}>{error}</Text>}
      </ScrollView>
    </View>
  );
}
