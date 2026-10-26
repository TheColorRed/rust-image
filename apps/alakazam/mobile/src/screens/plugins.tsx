import { AppText } from '@/src/components/app-text';
import { Card, CardHeader } from '@/src/components/card';
import { AiProgressBar } from '@/src/components/editor/ai-progress-bar';
import { MagicalSlogan } from '@/src/components/magical-slogan';
import {
  deleteModel,
  deletePluginListing,
  deletePlugins,
  downloadModel,
  downloadPlugin,
  pluginsToRemove,
  refreshAiModels,
  syncAiModels,
} from '@/src/lib/ai/ai-model-catalog';
import { loadPlugins } from '@/src/lib/plugins/plugin-controls';
import { useCommonStyles } from '@/src/lib/ui/styles';
import { useTheme } from '@/src/lib/ui/theme';
import { errorMessage } from '@/src/lib/utils/error-message';
import { formatBytes } from '@/src/lib/utils/format-bytes';
import { aiModelList, aiModelList$, type AiModelListing } from '@/src/state/ai-models';
import { pluginList, pluginList$, type PluginListing } from '@/src/state/plugins';
import { scanPlugins } from '@abra/core';
import { installPluginArchive, pluginsFolder } from '@alakazam/mobile';
import { errorCodes, isErrorWithCode, keepLocalCopy, pick, types } from '@react-native-documents/picker';
import ChevronDown from 'lucide-react-native/icons/chevron-down';
import ChevronRight from 'lucide-react-native/icons/chevron-right';
import Download from 'lucide-react-native/icons/download';
import RefreshCw from 'lucide-react-native/icons/refresh-cw';
import Trash2 from 'lucide-react-native/icons/trash';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { ActivityIndicator, Alert, NativeModules, Pressable, ScrollView, StyleSheet, Switch, Text, View } from 'react-native';
import { useObservable } from 'react-rx';

const removalMessage = (name: string, targetId: string, plan: ReturnType<typeof pluginsToRemove>) => {
  const dependents = plan.filter(plugin => plugin.id !== targetId);
  return dependents.length
    ? `${name} is required by ${dependents.map(plugin => plugin.name).join(', ')}. Deleting it will also uninstall ${dependents.length === 1 ? 'this add-on' : 'these add-ons'}.`
    : `Delete ${name}?`;
};

type PluginDevServerState = { running: boolean; port: number; token: string; addresses: string[] };
type PluginDevServerModule = {
  getPluginDevServerState(): Promise<PluginDevServerState>;
  startPluginDevServer(): Promise<PluginDevServerState>;
  stopPluginDevServer(): Promise<void>;
};

export function PluginsScreen() {
  const { colors } = useTheme();
  const commonStyles = useCommonStyles();
  const models = useObservable(aiModelList$, aiModelList.value);
  const addons = useObservable(pluginList$, pluginList.value);
  const installedPlugins = scanPlugins(pluginsFolder()).plugins;
  const installedPluginsById = new Map(installedPlugins.map(plugin => [plugin.id, plugin]));
  const [downloading, setDownloading] = useState<string | null>(null);
  /** The downloads the bar shows, by model: 0 waiting, null on its way, 1 done. Kept apart from the editor's AI work, which the editor reacts to. */
  const [downloadTasks, setDownloadTasks] = useState<Record<string, number | null>>({});
  const [error, setError] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(true);
  const [catalogLoaded, setCatalogLoaded] = useState(false);
  const [expanded, setExpanded] = useState<ReadonlySet<string>>(new Set());
  const [devServer, setDevServer] = useState<PluginDevServerState | null>(null);
  const [devServerBusy, setDevServerBusy] = useState(false);
  const styles = useMemo(
    () =>
      StyleSheet.create({
        container: { flex: 1, backgroundColor: colors.background, paddingTop: 24 },
        content: { paddingHorizontal: 16, paddingTop: 12, paddingBottom: 48, gap: 12 },
        flatRow: { paddingLeft: 16 },
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
        actions: { flexDirection: 'row', alignItems: 'center' },
        progressButton: {
          height: 36,
          paddingVertical: 0,
          paddingHorizontal: 14,
          justifyContent: 'center',
          borderWidth: 1,
          borderColor: colors.accent,
          overflow: 'hidden',
        },
        progressFill: { ...StyleSheet.absoluteFill, opacity: 0.45 },
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
    setCatalogLoaded(false);
    syncAiModels();
    try {
      await refreshAiModels();
      setCatalogLoaded(true);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setRefreshing(false);
    }
  }, []);
  useEffect(() => {
    reload();
  }, [reload]);

  useEffect(() => {
    const server = NativeModules.MediaFile as PluginDevServerModule | undefined;
    if (!server?.getPluginDevServerState) return;
    server.getPluginDevServerState().then(setDevServer).catch(e => setError(errorMessage(e)));
  }, []);

  const toggleDevServer = async () => {
    const server = NativeModules.MediaFile as PluginDevServerModule | undefined;
    if (!server) { setError('The plugin development server is unavailable on this device.'); return; }
    setDevServerBusy(true);
    setError(null);
    try {
      if (devServer?.running) {
        await server.stopPluginDevServer();
        setDevServer(await server.getPluginDevServerState());
      } else {
        setDevServer(await server.startPluginDevServer());
      }
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setDevServerBusy(false);
    }
  };

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

  /** Clears the bar a moment after every download is done, so a short one is seen. */
  const clearWhenDone = () =>
    setTimeout(
      () => setDownloadTasks(current => (Object.values(current).every(progress => progress === 1) ? {} : current)),
      500,
    );

  /** Moves a model's share of the bar as the download reports how far it has got. */
  const downloadListener = (model: AiModelListing) => ({
    update: (progress: number) => setDownloadTasks(current => ({ ...current, [model.id]: progress })),
  });

  const download = async (model: AiModelListing) => {
    setError(null);
    setDownloading(model.id);
    setDownloadTasks({ [model.id]: null });
    try {
      await downloadModel(model, downloadListener(model));
      setDownloadTasks({ [model.id]: 1 });
      clearWhenDone();
    } catch (e) {
      setError(errorMessage(e));
      setDownloadTasks({});
    }
    setDownloading(null);
    syncAiModels();
  };

  const remove = (model: AiModelListing) => {
    setError(null);
    if (model.source.type === 'plugin') {
      const plan = pluginsToRemove([model.source.pluginId]);
      Alert.alert('Delete plugin?', removalMessage(model.title, model.source.pluginId, plan), [
        { text: 'Cancel', style: 'cancel' },
        {
          text: plan.length > 1 ? 'Delete all' : 'Delete',
          style: 'destructive',
          onPress: () => {
            try {
              deleteModel(
                model,
                plan.map(plugin => plugin.id),
              );
            } catch (e) {
              setError(errorMessage(e));
            }
            syncAiModels();
          },
        },
      ]);
      return;
    }
    try {
      deleteModel(model);
    } catch (e) {
      setError(errorMessage(e));
    }
    syncAiModels();
  };

  /** Installs a plugin that adds no model, and the plugins it depends on. Its row in the list is keyed `plugin:<id>`. */
  const downloadAddon = async (plugin: PluginListing) => {
    const key = `plugin:${plugin.id}`;
    setError(null);
    setDownloading(key);
    setDownloadTasks({ [key]: null });
    try {
      await downloadPlugin(plugin, {
        update: progress => setDownloadTasks(current => ({ ...current, [key]: progress })),
      });
      setDownloadTasks({ [key]: 1 });
      clearWhenDone();
    } catch (e) {
      setError(errorMessage(e));
      setDownloadTasks({});
    }
    setDownloading(null);
    syncAiModels();
  };

  /** Installs a plugin package built locally by its author. */
  const installLocal = async () => {
    setError(null);
    setDownloading('local-plugin');
    setDownloadTasks({ 'local-plugin': null });
    try {
      const [picked] = await pick({ type: [types.zip] });
      if (!picked.name?.endsWith('.akzx'))
        throw new Error('Choose an .akzx package built with @alakazam/plugin.');
      const [copy] = await keepLocalCopy({
        files: [{ uri: picked.uri, fileName: picked.name }],
        destination: 'cachesDirectory',
      });
      if (copy.status !== 'success') throw new Error(`Could not read the package: ${copy.copyError}`);
      const archivePath = decodeURIComponent(copy.localUri.replace(/^file:\/\//, ''));
      await installPluginArchive(archivePath, { update: progress => setDownloadTasks({ 'local-plugin': progress }) });
      loadPlugins();
      syncAiModels();
      setDownloadTasks({ 'local-plugin': 1 });
      clearWhenDone();
    } catch (e) {
      if (isErrorWithCode(e) && e.code === errorCodes.OPERATION_CANCELED) {
        setDownloadTasks({});
      } else {
        setError(errorMessage(e));
        setDownloadTasks({});
      }
    }
    setDownloading(null);
  };

  const removeAddon = (plugin: PluginListing) => {
    setError(null);
    const plan = pluginsToRemove([plugin.id]);
    Alert.alert('Delete plugin?', removalMessage(plugin.name, plugin.id, plan), [
      { text: 'Cancel', style: 'cancel' },
      {
        text: plan.length > 1 ? 'Delete all' : 'Delete',
        style: 'destructive',
        onPress: () => {
          try {
            deletePluginListing(
              plugin,
              plan.map(item => item.id),
            );
          } catch (e) {
            setError(errorMessage(e));
          }
          syncAiModels();
        },
      },
    ]);
  };

  const removeInstalledPlugin = (id: string) => {
    setError(null);
    const plan = pluginsToRemove([id]);
    const name = installedPlugins.find(plugin => plugin.id === id)?.name ?? id;
    Alert.alert('Delete plugin?', removalMessage(name, id, plan), [
      { text: 'Cancel', style: 'cancel' },
      {
        text: plan.length > 1 ? 'Delete all' : 'Delete',
        style: 'destructive',
        onPress: () => {
          try {
            deletePlugins(plan.length ? plan.map(plugin => plugin.id) : [id]);
            syncAiModels();
          } catch (e) {
            setError(errorMessage(e));
          }
        },
      },
    ]);
  };

  const listedPluginIds = new Set([
    ...addons.map(plugin => plugin.id),
    ...models.flatMap(model => (model.source.type === 'plugin' ? [model.source.pluginId] : [])),
  ]);
  // Once the catalog has loaded, this card only contains packages not already represented by
  // an add-on or an AI model. When it cannot load, keep every installed package manageable.
  const installedPluginRows = catalogLoaded
    ? installedPlugins.filter(plugin => !listedPluginIds.has(plugin.id))
    : installedPlugins;
  const hasPluginUpdate = (id: string, version: string) => {
    const installed = installedPluginsById.get(id);
    return installed !== undefined && installed.version !== version;
  };

  /** Smallest first, so more models are ready sooner and little is lost if the connection drops. */
  const notDownloaded = shown
    .filter(model => model.enabled && !model.downloaded)
    .sort((a, b) => a.downloadSizeBytes - b.downloadSizeBytes);
  const downloaded = shown.filter(model => model.downloaded);
  const notDownloadedBytes = notDownloaded.reduce((total, model) => total + model.downloadSizeBytes, 0);
  const downloadedBytes = downloaded.reduce((total, model) => total + model.sizeBytes, 0);

  /** Every shown model that is not on this device yet, one after another, stopping at the first that fails. */
  const downloadAll = () =>
    Alert.alert('Download all AI models?', `This downloads ${formatBytes(notDownloadedBytes)}.`, [
      { text: 'Cancel', style: 'cancel' },
      {
        text: 'Download all',
        onPress: async () => {
          setError(null);
          setDownloadTasks(Object.fromEntries(notDownloaded.map(model => [model.id, 0])));
          for (const model of notDownloaded) {
            setDownloading(model.id);
            setDownloadTasks(current => ({ ...current, [model.id]: null }));
            try {
              await downloadModel(model, downloadListener(model));
            } catch (e) {
              setError(errorMessage(e));
              setDownloadTasks({});
              break;
            }
            setDownloadTasks(current => ({ ...current, [model.id]: 1 }));
            syncAiModels();
          }
          clearWhenDone();
          setDownloading(null);
          syncAiModels();
        },
      },
    ]);

  const removeAll = () => {
    const downloadedModels = shown.filter(model => model.downloaded);
    const targetPluginIds = new Set(
      downloadedModels.flatMap(model => (model.source.type === 'plugin' ? [model.source.pluginId] : [])),
    );
    const dependentAddons = pluginsToRemove([...targetPluginIds]).filter(plugin => !targetPluginIds.has(plugin.id));
    const cascadeNote = dependentAddons.length
      ? ` This will also uninstall required add-ons: ${dependentAddons.map(plugin => plugin.name).join(', ')}.`
      : '';
    Alert.alert(
      'Delete all AI models?',
      `This frees ${formatBytes(downloadedBytes)}. They download again when an edit needs them, and saved projects still open without them.${cascadeNote}`,
      [
        { text: 'Cancel', style: 'cancel' },
        {
          text: 'Delete all',
          style: 'destructive',
          onPress: () => {
            setError(null);
            for (const model of downloadedModels) {
              try {
                deleteModel(model);
              } catch (e) {
                setError(errorMessage(e));
                break;
              }
            }
            syncAiModels();
          },
        },
      ],
    );
  };

  /** One row of a list: what it is, whether it is on this device, and the button for it. */
  const renderRow = (item: {
    id: string;
    title: string;
    description: string;
    downloaded: boolean;
    sizeBytes: number;
    downloadSizeBytes: number;
    statusText?: string;
    updateAvailable?: boolean;
    /** Without the indent of a row under a group. */
    flat?: boolean;
    onDownload: () => void;
    onDelete: () => void;
  }) => (
    <View key={item.id} style={[styles.row, item.flat && styles.flatRow]}>
      <View style={styles.rowText}>
        <Text style={styles.rowTitle}>{item.title}</Text>
        {item.description !== '' && <Text style={styles.rowSize}>{item.description}</Text>}
        <Text style={styles.rowSize}>
          {item.statusText ?? (item.updateAvailable
            ? 'Update available'
            : item.downloaded
              ? `Downloaded · ${formatBytes(item.sizeBytes)}`
              : `Not downloaded · ${formatBytes(item.downloadSizeBytes)}`)}
        </Text>
      </View>
      {item.id in downloadTasks && !item.downloaded ? (
        <View style={[styles.button, styles.progressButton]}>
          <View pointerEvents="none" style={styles.progressFill}>
            <AiProgressBar
              tasks={{ [item.id]: downloadTasks[item.id] }}
              label={`Downloading ${item.title}`}
              height={36}
              radius={8}
              bare
              pulse={false}
            />
          </View>
          <Text style={[styles.buttonText, { color: colors.textPrimary }]}>
            {downloading === item.id ? 'Downloading' : 'Waiting'}
          </Text>
        </View>
      ) : item.downloaded ? (
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={`Delete ${item.title}`}
          onPress={() => item.onDelete()}
          style={[styles.button, styles.remove]}
        >
          <Text style={[styles.buttonText, { color: colors.danger }]}>Delete</Text>
        </Pressable>
      ) : (
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={`${item.updateAvailable ? 'Update' : 'Download'} ${item.title}`}
          disabled={downloading !== null}
          onPress={() => item.onDownload()}
          style={[styles.button, styles.download, downloading !== null && { opacity: 0.4 }]}
        >
          <Text style={[styles.buttonText, { color: '#fff' }]}>{item.updateAvailable ? 'Update' : 'Download'}</Text>
        </Pressable>
      )}
    </View>
  );

  return (
    <View style={styles.container}>
      <View style={[commonStyles.header, { alignItems: 'center' }]}>
        <AppText size="heading" bold>
          Plugins
        </AppText>
        <Pressable
          accessibilityRole="button"
          accessibilityLabel="Refresh plugins"
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
      </View>
      <AiProgressBar tasks={downloadTasks} label="Downloading plugins and AI models" pulse={false} />

      <ScrollView contentContainerStyle={styles.content}>
        {refreshing ? (
          <View style={{ minHeight: 240, justifyContent: 'center', alignItems: 'center' }}>
            <MagicalSlogan fontSize={32} subText="Loading plugins..." />
          </View>
        ) : (
          <>
            <Card
              title="Local plugin package"
              subtitle="Install a plugin package from your device. Its files are checked against plugin.yml before installation."
            >
              <Pressable
                accessibilityRole="button"
                accessibilityLabel="Install local plugin package"
                disabled={downloading !== null}
                onPress={installLocal}
                style={[styles.row, { paddingLeft: 16, borderTopWidth: 0 }, downloading !== null && { opacity: 0.5 }]}
              >
                <View style={styles.rowText}>
                  <Text style={styles.rowTitle}>
                    {downloading === 'local-plugin' ? 'Installing plugin' : 'Install from file'}
                  </Text>
                  <Text style={styles.rowSize}>Open a local plugin file to install</Text>
                </View>
                {downloading === 'local-plugin' ? (
                  <ActivityIndicator color={colors.accent} />
                ) : (
                  <Download size={20} color={colors.accent} />
                )}
              </Pressable>
            </Card>
            {addons.some(plugin => plugin.enabled || plugin.downloaded) && (
              <Card title="Add-ons">
                <CardHeader title="Add-ons" />
                {addons
                  .filter(plugin => plugin.enabled || plugin.downloaded)
                  .map((plugin, index) =>
                    renderRow({
                      id: `plugin:${plugin.id}`,
                      title: plugin.name,
                      description: plugin.description,
                      downloaded: plugin.downloaded,
                      updateAvailable: hasPluginUpdate(plugin.id, plugin.version),
                      sizeBytes: plugin.downloadSizeBytes,
                      downloadSizeBytes: plugin.downloadSizeBytes,
                      flat: true,
                      first: index === 0,
                      onDownload: () => downloadAddon(plugin),
                      onDelete: () => removeAddon(plugin),
                    }),
                  )}
              </Card>
            )}
            <Card title="AI models">
              <CardHeader
                title="AI models"
                action={
                  <View style={styles.actions}>
                    {notDownloaded.length > 0 && (
                      <Pressable
                        accessibilityRole="button"
                        accessibilityLabel="Download all AI models"
                        disabled={downloading !== null}
                        hitSlop={8}
                        onPress={downloadAll}
                        style={[styles.refresh, downloading !== null && { opacity: 0.4 }]}
                      >
                        <Download size={22} color={colors.success} strokeWidth={1.8} />
                      </Pressable>
                    )}
                    {downloaded.length > 0 && (
                      <Pressable
                        accessibilityRole="button"
                        accessibilityLabel="Delete all AI models"
                        disabled={downloading !== null}
                        hitSlop={8}
                        onPress={removeAll}
                        style={[styles.refresh]}
                      >
                        <Trash2 size={22} color={colors.danger} strokeWidth={1.8} />
                      </Pressable>
                    )}
                  </View>
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
                    members.map(model =>
                      renderRow({
                        id: model.id,
                        title: model.title,
                        description: model.description,
                        downloaded: model.downloaded,
                        updateAvailable:
                          model.source.type === 'plugin' &&
                          hasPluginUpdate(model.source.pluginId, model.source.version),
                        sizeBytes: model.sizeBytes,
                        downloadSizeBytes: model.downloadSizeBytes,
                        onDownload: () => download(model),
                        onDelete: () => remove(model),
                      }),
                    )}
                </View>
              ))}
            </Card>
            <Card
              title="Other installed plugins"
              subtitle="Installed here but not listed above, including plugins removed from the catalog."
            >
              {installedPluginRows.length === 0 ? (
                <Text style={[styles.rowSize, { padding: 16 }]}>No other plugins are installed on this device.</Text>
              ) : installedPluginRows.map(plugin =>
                renderRow({
                  id: `installed:${plugin.id}`,
                  title: plugin.name,
                  description: `${plugin.id} · ${plugin.version}${plugin.missingDependencies.length ? ` · needs ${plugin.missingDependencies.join(', ')}` : ''}`,
                  downloaded: true,
                  sizeBytes: 0,
                  downloadSizeBytes: 0,
                  statusText: 'Installed on this device',
                  flat: true,
                  onDownload: () => {},
                  onDelete: () => removeInstalledPlugin(plugin.id),
                }),
              )}
            </Card>
            {error && <Text style={styles.error}>{error}</Text>}
            <Card title="Developer" subtitle="Sync plugin changes from your computer over the local network.">
              <View style={{ flexDirection: 'row', alignItems: 'center', padding: 16, gap: 12 }}>
                <View style={styles.rowText}>
                  <Text style={styles.rowTitle}>Development server</Text>
                  <Text style={styles.rowSize}>{devServer?.running ? 'On' : 'Off'}</Text>
                </View>
                {devServerBusy && <ActivityIndicator color={colors.accent} />}
                <Switch
                  accessibilityLabel="Plugin development server"
                  value={devServer?.running ?? false}
                  disabled={devServerBusy}
                  onValueChange={toggleDevServer}
                  trackColor={{ true: colors.accent }}
                />
              </View>
              {devServer?.running && (
                <View style={{ paddingHorizontal: 16, paddingBottom: 16, gap: 6 }}>
                  <Text style={styles.rowTitle}>Server addresses</Text>
                  {devServer.addresses.length ? devServer.addresses.map(address => (
                    <Text key={address} selectable style={styles.rowSize}>{address}</Text>
                  )) : <Text style={styles.rowSize}>No local network address found.</Text>}
                  <Text style={[styles.rowTitle, { marginTop: 8 }]}>Access token</Text>
                  <Text selectable style={styles.rowSize}>{devServer.token}</Text>
                  <Text style={[styles.rowSize, { marginTop: 6 }]}>Keep the token private and use a trusted network.</Text>
                </View>
              )}
            </Card>
          </>
        )}
      </ScrollView>
    </View>
  );
}
