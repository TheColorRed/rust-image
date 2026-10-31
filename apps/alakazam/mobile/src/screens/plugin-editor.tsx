import { EditorToolbar } from '@/src/components/editor/editor-toolbar';
import { ViewControls } from '@/src/components/editor/view-controls';
import { Inspector } from '@/src/components/plugin-editor/inspector/inspector';
import { SelectionBar, useSelection } from '@/src/components/selection';
import { WorkspacePreview } from '@/src/components/workspace-preview';
import { openPluginWorkspace, updatePluginFiles, type PluginWorkspaceInfo } from '@/src/lib/plugins/plugin-workspace';
import { useTheme } from '@/src/lib/ui/theme';
import {
  pluginEditorBuilding$,
  pluginEditorBuilding,
  pluginEditorCanvas,
  pluginEditorCanvas$,
  pluginEditorDevUpdate$,
  pluginEditorHasControls,
  pluginEditorHasControls$,
  pluginEditorHasInspector,
  pluginEditorHasInspector$,
  pluginEditorInspectorOpen,
  pluginEditorInspectorOpen$,
  pluginEditorItems,
  pluginEditorItems$,
  pluginEditorPicked,
  pluginEditorPreviewError,
  pluginEditorPreviewError$,
} from '@/src/state/plugin-editor';
import { ExportDialog, type ExportDialogItem } from '@/src/components/export-dialog';
import { cacheFileUri, shareImageFile, writeImageToGallery } from '@/src/lib/native/media-file';
import { errorMessage } from '@/src/lib/utils/error-message';
import { release } from '@/src/lib/native/native-handles';
import { exportFormat, exportQuality, type ExportFormat } from '@/src/state/settings';
import ListSortDescending from 'lucide-react-native/icons/list-sort-descending';
import Save from 'lucide-react-native/icons/save';
import Share2 from 'lucide-react-native/icons/share-2';
import X from 'lucide-react-native/icons/x';
import { useEffect, useMemo, useRef, useState } from 'react';
import { useObservable } from 'react-rx';
import { ActivityIndicator, Alert, Pressable, StyleSheet, Text, useWindowDimensions, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';

export type PluginWorkspaceScreenInfo = PluginWorkspaceInfo;

/** A minimal host layout for plugin-authored image tools and their inspector controls. */
export function PluginEditorScreen({
  workspace,
  onClose,
}: {
  workspace: PluginWorkspaceScreenInfo;
  onClose: () => void;
}) {
  const { colors } = useTheme();
  const { width } = useWindowDimensions();
  const insets = useSafeAreaInsets();
  const inspectorOpen = useObservable(pluginEditorInspectorOpen$, pluginEditorInspectorOpen.value);
  const hasInspector = useObservable(pluginEditorHasInspector$, pluginEditorHasInspector.value);
  const hasControls = useObservable(pluginEditorHasControls$, pluginEditorHasControls.value);
  const inspectorItems = useObservable(pluginEditorItems$, pluginEditorItems.value);
  const workspaceCanvas = useObservable(pluginEditorCanvas$, pluginEditorCanvas.value);
  const previewError = useObservable(pluginEditorPreviewError$, pluginEditorPreviewError.value);
  const building = useObservable(pluginEditorBuilding$, pluginEditorBuilding.value);
  const selection = useSelection();
  const wideLayout = width >= 700;
  const [hotUpdate, setHotUpdate] = useState(0);
  const [saveOpen, setSaveOpen] = useState(false);
  const [sharing, setSharing] = useState(false);
  const [exportUri, setExportUri] = useState('');
  const exportItems = useMemo(() => [{ id: 'workspace', uri: exportUri, name: `${workspace.title}.jpg` }], [exportUri, workspace.title]);

  /** The save menu shows its picture from a file, so the canvas on show is written to the cache once, as the menu opens. */
  const openSave = async () => {
    const canvas = pluginEditorCanvas.value;
    if (!canvas) return;
    const image = canvas.asImage();
    try {
      const uri = await cacheFileUri(`${workspace.title}-preview.jpg`);
      image.write(decodeURIComponent(uri.replace(/^file:\/\//, '')), { quality: 90 });
      setExportUri(uri);
      setSaveOpen(true);
    } catch (failure) {
      Alert.alert('Could not open the save menu', errorMessage(failure));
    } finally {
      release(image);
    }
  };

  /** Writes the canvas on show to the cache in the saved format and quality, and opens the phone's share sheet with it. */
  const shareWorkspace = async () => {
    const canvas = pluginEditorCanvas.value;
    if (!canvas || sharing) return;
    setSharing(true);
    const image = canvas.asImage();
    try {
      const uri = await cacheFileUri(`${workspace.title}.${exportFormat.value}`);
      image.write(decodeURIComponent(uri.replace(/^file:\/\//, '')), { quality: exportQuality.value });
      // Not deleted here: the app picked in the share sheet reads the file later.
      await shareImageFile(uri);
    } catch (failure) {
      Alert.alert('Could not share the picture', errorMessage(failure));
    } finally {
      release(image);
      setSharing(false);
    }
  };

  /** Flattens the canvas on show once, writes it where the editor's save does, and frees the flattened picture. */
  const saveWorkspace = async (_item: ExportDialogItem, fileName: string, format: ExportFormat, quality: number) => {
    const canvas = pluginEditorCanvas.value;
    if (!canvas) throw new Error('There is nothing to save yet.');
    const image = canvas.asImage();
    try {
      await writeImageToGallery(image, fileName, format, quality);
    } finally {
      release(image);
    }
  };

  useEffect(() => {
    const subscription = pluginEditorDevUpdate$.subscribe(pluginId => {
      if (pluginId === workspace.pluginId) setHotUpdate(value => value + 1);
    });
    return () => subscription.unsubscribe();
  }, [workspace.pluginId]);

  // The workspace is run by the host (see `plugin-workspace`); this screen only lays out what it made.
  // The project this workspace saves into. It starts as the one that was opened, or empty until the first picture is picked.
  const project = useRef(workspace.project);
  useEffect(
    () => openPluginWorkspace(workspace, project),
    [workspace.entryPath, workspace.pluginId, workspace.title, hotUpdate],
  );

  /** Takes the selected thumbnails out of every control's list and tells the plugin the lists that are left. */
  const removeSelected = () => {
    for (const item of inspectorItems) {
      if (item.kind !== 'file-picker') continue;
      const files = pluginEditorPicked.value[item.key] ?? [];
      const remaining = files.filter(file => !selection.selected.includes(file.uri));
      if (item.thumbs && remaining.length !== files.length) updatePluginFiles(item, remaining);
    }
    selection.clear();
  };

  const inspector = (
    <View style={[styles.inspector, { borderColor: colors.border, backgroundColor: colors.surface }]}>
      <Inspector surface="inspector" selection={selection} onFiles={updatePluginFiles} />
    </View>
  );

  return (
    <View style={[styles.screen, { backgroundColor: colors.background }]}>
      <EditorToolbar
        onClose={onClose}
        rightActions={
          <>
            {workspaceCanvas ? (
              <Pressable
                accessibilityRole="button"
                accessibilityLabel="Save to device"
                style={styles.inspectorButton}
                onPress={openSave}
                hitSlop={8}
              >
                <Save size={22} color="#fff" />
              </Pressable>
            ) : null}
            {workspaceCanvas ? (
              <Pressable
                accessibilityRole="button"
                accessibilityLabel="Share"
                style={[styles.inspectorButton, sharing && { opacity: 0.5 }]}
                disabled={sharing}
                onPress={shareWorkspace}
                hitSlop={8}
              >
                <Share2 size={22} color="#fff" />
              </Pressable>
            ) : null}
            {hasInspector && !wideLayout ? (
              <Pressable
                accessibilityRole="button"
                accessibilityLabel="Open inspector"
                accessibilityState={{ expanded: inspectorOpen }}
                style={styles.inspectorButton}
                onPress={() => pluginEditorInspectorOpen.next(!pluginEditorInspectorOpen.value)}
                hitSlop={8}
              >
                <ListSortDescending size={20} color="#fff" />
              </Pressable>
            ) : null}
          </>
        }
      />
      <ExportDialog
        items={exportItems}
        visible={saveOpen}
        onClose={() => setSaveOpen(false)}
        onSaveImage={saveWorkspace}
      />

      <View style={styles.workspace}>
        <View style={styles.previewColumn}>
          <View style={[styles.preview, { backgroundColor: colors.placeholder }]}>
            {workspaceCanvas ? (
              <WorkspacePreview canvas={workspaceCanvas} />
            ) : (
              <Text style={[styles.label, { color: colors.textSecondary }]}>Image Preview</Text>
            )}
            {building ? <ActivityIndicator style={styles.building} color={colors.accent} /> : null}
            {previewError ? (
              <Text style={[styles.previewError, { color: colors.danger }]}>{previewError}</Text>
            ) : null}
          </View>
          <View style={[styles.toolsBar, { borderColor: colors.border }]}>
            <ViewControls placement="bar" />
          </View>
        </View>
        {wideLayout && hasInspector ? inspector : null}
      </View>

      <>
        {hasInspector && !wideLayout && inspectorOpen ? (
          <View style={styles.drawerLayer}>
            <Pressable
              accessibilityRole="button"
              accessibilityLabel="Close inspector"
              style={styles.backdrop}
              onPress={() => pluginEditorInspectorOpen.next(false)}
            />
            <View
              style={[
                styles.drawer,
                {
                  bottom: insets.bottom,
                  width: Math.min(width * 0.86, 360),
                  borderColor: colors.border,
                  backgroundColor: colors.surface,
                },
              ]}
            >
              <View style={[styles.drawerHeader, { borderColor: colors.border }]}>
                <Text style={[styles.label, { color: colors.textPrimary }]}>Inspector</Text>
                <Pressable
                  accessibilityRole="button"
                  accessibilityLabel="Close inspector"
                  onPress={() => pluginEditorInspectorOpen.next(false)}
                  style={styles.closeButton}
                  hitSlop={8}
                >
                  <X size={24} color={colors.textPrimary} />
                </Pressable>
              </View>
              <View style={styles.drawerContent}><Inspector surface="inspector" selection={selection} onFiles={updatePluginFiles} /></View>
            </View>
          </View>
        ) : null}
      </>

      {hasControls ? (
        <View style={[styles.controlBar, { borderColor: colors.border, backgroundColor: colors.surface }]}>
          <Inspector surface="controls" selection={selection} onFiles={updatePluginFiles} />
        </View>
      ) : null}

      {selection.selecting && (
        <View pointerEvents="box-none" style={styles.selectionLayer}>
          <SelectionBar
            count={selection.selected.length}
            actionLabel="Remove"
            destructive
            onCancel={selection.clear}
            onAction={removeSelected}
          />
        </View>
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  screen: { flex: 1 },
  workspace: { flex: 1, flexDirection: 'row', minHeight: 0 },
  previewColumn: { flex: 3, minWidth: 0 },
  preview: { flex: 1, alignItems: 'center', justifyContent: 'center' },
  previewError: { position: 'absolute', left: 12, right: 12, bottom: 12 },
  building: { position: 'absolute', top: 12, right: 12 },
  toolsBar: { minHeight: 48, borderTopWidth: StyleSheet.hairlineWidth },
  inspector: {
    flex: 1,
    minWidth: 120,
    paddingTop: 56,
    borderLeftWidth: StyleSheet.hairlineWidth,
    alignItems: 'center',
    justifyContent: 'center',
  },
  inspectorButton: {
    width: 40,
    height: 40,
    borderRadius: 20,
    backgroundColor: 'rgba(0,0,0,0.45)',
    alignItems: 'center',
    justifyContent: 'center',
  },
  selectionLayer: { ...StyleSheet.absoluteFill, zIndex: 30, elevation: 30 },
  drawerLayer: { position: 'absolute', left: 0, right: 0, top: 0, bottom: 0, zIndex: 20, elevation: 20 },
  backdrop: { ...StyleSheet.absoluteFill, backgroundColor: 'rgba(0,0,0,0.4)' },
  drawer: { position: 'absolute', top: 0, right: 0, borderLeftWidth: StyleSheet.hairlineWidth },
  closeButton: { width: 44, height: 44, alignItems: 'center', justifyContent: 'center' },
  drawerHeader: {
    minHeight: 56,
    paddingHorizontal: 16,
    borderBottomWidth: StyleSheet.hairlineWidth,
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
  },
  drawerContent: { flex: 1 },
  controlBar: {
    height: 160,
    borderTopWidth: StyleSheet.hairlineWidth,
  },
  label: { fontSize: 16, fontWeight: '600' },
});
