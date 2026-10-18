import { AppText } from '@/src/components/app-text';
import { Card, CardHeader } from '@/src/components/card';
import { errorMessage } from '@/src/lib/error-message';
import { formatBytes } from '@/src/lib/format-bytes';
import { useCommonStyles } from '@/src/lib/styles';
import { useTheme } from '@/src/lib/theme';
import { refreshAiModels, syncAiModels } from '@/src/lib/ai-model-catalog';
import { aiModelList, aiModelList$, type AiModelListing } from '@/src/state/ai-models';
import { deleteAiModel, downloadAiModel } from '@alakazam/mobile';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { ChevronDown, ChevronRight, RefreshCw } from 'lucide-react-native';
import { ActivityIndicator, Pressable, ScrollView, StyleSheet, Text, View } from 'react-native';
import { useObservable } from 'react-rx';

export function StorageScreen() {
  const { colors } = useTheme();
  const commonStyles = useCommonStyles();
  const models = useObservable(aiModelList$, aiModelList.value);
  const [downloading, setDownloading] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [expanded, setExpanded] = useState<ReadonlySet<string>>(new Set());
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
          paddingLeft: 56,
          paddingRight: 16,
          paddingVertical: 12,
          borderTopWidth: 1,
          borderTopColor: colors.border,
        },
        group: {
          flexDirection: 'row',
          alignItems: 'center',
          gap: 8,
          paddingHorizontal: 16,
          paddingVertical: 12,
          borderTopWidth: 1,
          borderTopColor: colors.border,
        },
        groupTitle: { flex: 1, fontSize: 15, fontWeight: '600', color: colors.textSecondary },
        rowText: { flex: 1 },
        rowTitle: { fontSize: 16, color: colors.textPrimary },
        rowSize: { marginTop: 2, fontSize: 13, color: colors.textSecondary },
        button: { minWidth: 96, paddingVertical: 8, borderRadius: 8, alignItems: 'center' },
        download: { backgroundColor: colors.accent },
        remove: { borderWidth: 1, borderColor: colors.danger },
        buttonText: { fontSize: 14, fontWeight: '600' },
        refresh: { width: 40, height: 40, alignItems: 'center', justifyContent: 'center', borderRadius: 20 },
        note: { fontSize: 13, color: colors.textSecondary },
        error: { fontSize: 13, color: colors.danger },
      }),
    [colors],
  );

  // The list shown is the one kept from last time, with what is on this device brought up to date at once. The list from S3
  // is fetched each time the tab opens, and again when the refresh button is pressed, and replaces it.
  const reload = useCallback(async () => {
    setError(null);
    setRefreshing(true);
    syncAiModels();
    try {
      await refreshAiModels();
    } catch (e) {
      setError(errorMessage(e));
    }
    setRefreshing(false);
  }, []);
  useEffect(() => {
    reload();
  }, [reload]);

  /** A model that is turned off is hidden unless it is on this device, so it can still be deleted to free the space. */
  const shown = models.filter(model => model.enabled || model.downloaded);

  /** The shown models by the first subject the catalog gives them, in the order the subjects first appear. A model with none is under Other. */
  const groups = new Map<string, AiModelListing[]>();
  for (const model of shown) {
    const subject = model.subjects[0] ?? 'other';
    groups.set(subject, [...(groups.get(subject) ?? []), model]);
  }
  const toggle = (subject: string) =>
    setExpanded(current => {
      const next = new Set(current);
      if (!next.delete(subject)) next.add(subject);
      return next;
    });

  const download = async (model: AiModelListing) => {
    setError(null);
    setDownloading(model.id);
    try {
      await downloadAiModel(model.file);
    } catch (e) {
      setError(errorMessage(e));
    }
    setDownloading(null);
    syncAiModels();
  };

  const remove = (model: AiModelListing) => {
    setError(null);
    try {
      deleteAiModel(model.id);
    } catch (e) {
      setError(errorMessage(e));
    }
    syncAiModels();
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
          <CardHeader
            title="AI models"
            action={
              <Pressable
                accessibilityRole="button"
                accessibilityLabel="Refresh AI models"
                disabled={refreshing}
                hitSlop={8}
                onPress={reload}
                style={styles.refresh}
              >
                {refreshing ? (
                  <ActivityIndicator color={colors.accent} />
                ) : (
                  <RefreshCw size={22} color={colors.accent} strokeWidth={1.8} />
                )}
              </Pressable>
            }
          />
          {[...groups].map(([subject, members]) => (
            <View key={subject}>
              <Pressable
                accessibilityRole="button"
                accessibilityState={{ expanded: expanded.has(subject) }}
                accessibilityLabel={`${subject} models`}
                onPress={() => toggle(subject)}
                style={styles.group}
              >
                {!expanded.has(subject) ? (
                  <ChevronRight size={20} color={colors.textSecondary} />
                ) : (
                  <ChevronDown size={20} color={colors.textSecondary} />
                )}
                <Text style={styles.groupTitle}>{subject.charAt(0).toUpperCase() + subject.slice(1)}</Text>
              </Pressable>
              {expanded.has(subject) &&
                members.map(model => (
                  <View key={model.id} style={styles.row}>
                    <View style={styles.rowText}>
                      <Text style={styles.rowTitle}>{model.title}</Text>
                      {model.description !== '' && <Text style={styles.rowSize}>{model.description}</Text>}
                      <Text style={styles.rowSize}>
                        {model.downloaded
                          ? `Downloaded · ${formatBytes(model.sizeBytes)}`
                          : `Not downloaded · ${formatBytes(model.downloadSizeBytes)}`}
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
