import { Card, CardHeader } from '@/src/components/card';
import { ExportDialog, type ExportDialogItem } from '@/src/components/export-dialog';
import { FilePicker } from '@/src/components/file-picker';
import { MagicalSlogan } from '@/src/components/magical-slogan';
import {
  SelectableThumb,
  SELECTION_BAR_CLEARANCE,
  SelectionBar,
  SelectionPreview,
  useSelection,
} from '@/src/components/selection';
import { useCameraPhotos } from '@/src/hooks/useCameraPhotos';
import { toLocalPath } from '@/src/lib/editor/edit-source';
import { writeImageToGallery } from '@/src/lib/native/media-file';
import { release } from '@/src/lib/native/native-handles';
import { projectImageUri, projectSourceUri, type Project } from '@/src/lib/project/project-store';
import { useCommonStyles } from '@/src/lib/ui/styles';
import { useTheme } from '@/src/lib/ui/theme';
import type { EditablePhoto } from '@/src/screens/editor';
import type { PluginWorkspaceScreenInfo } from '@/src/screens/plugin-editor';
import { addProject, projects$, removeProjects } from '@/src/state/projects';
import type { ExportFormat } from '@/src/state/settings';
import { Image as AbraPixels } from '@abra/core/image';
import { pluginsFolder, scanPlugins } from '@alakazam/mobile';
import type { PhotoIdentifier } from '@react-native-camera-roll/camera-roll';
import CameraIcon from 'lucide-react-native/icons/camera';
import Blocks from 'lucide-react-native/icons/blocks';
import { useMemo, useState } from 'react';
import {
  ActivityIndicator,
  Alert,
  FlatList,
  Image,
  Linking,
  RefreshControl,
  ScrollView,
  StyleSheet,
  Text,
  TouchableOpacity,
  useWindowDimensions,
  View,
} from 'react-native';
import { useObservable } from 'react-rx';

/** The fewest columns, as in a portrait phone. A wider window fits more so the thumbnails stay about the same size. */
const MIN_COLUMNS = 3;
/** The smallest a thumbnail is allowed to be, in dp, before another column is added. */
const MIN_THUMB = 120;
const GAP = 2;

function Permissions({
  permission,
  photos,
  refreshing,
  loadMore,
  refresh,
  loadingMore,
  selection,
}: {
  permission: 'checking' | 'denied' | 'granted';
  photos: PhotoIdentifier[];
  refreshing: boolean;
  loadMore: () => void;
  refresh: () => void;
  loadingMore: boolean;
  selection: ReturnType<typeof useSelection>;
}) {
  const { colors } = useTheme();
  const commonStyles = useCommonStyles();
  // Follows the window, so turning the phone gives the grid the columns and thumbnails that fit the new width.
  const { width } = useWindowDimensions();
  const columns = Math.max(MIN_COLUMNS, Math.floor((width + GAP) / (MIN_THUMB + GAP)));
  const thumbSize = (width - GAP * (columns - 1)) / columns;
  const styles = useMemo(
    () =>
      StyleSheet.create({
        settingsButton: { marginTop: 8 },
        grid: { paddingHorizontal: 0 },
        row: { gap: GAP },
        thumbWrapper: { width: thumbSize, height: thumbSize, marginBottom: GAP },
        thumb: { width: '100%', height: '100%', backgroundColor: colors.placeholder },
        footer: { paddingVertical: 20 },
      }),
    [colors, thumbSize],
  );

  return (
    <>
      {permission === 'checking' && (
        <View style={commonStyles.centered}>
          <ActivityIndicator size="large" color={colors.accent} />
        </View>
      )}

      {permission === 'denied' && (
        <View style={commonStyles.centered}>
          <Text style={commonStyles.emptyTitle}>Photo access needed</Text>
          <Text style={commonStyles.emptyBody}>Grant access to your Camera Roll to see your photos here.</Text>
          <TouchableOpacity style={[commonStyles.button, styles.settingsButton]} onPress={() => Linking.openSettings()}>
            <Text style={commonStyles.buttonText}>Open Settings</Text>
          </TouchableOpacity>
        </View>
      )}

      {permission === 'granted' && photos.length === 0 && !refreshing && (
        <View style={commonStyles.centered}>
          <Text style={commonStyles.emptyTitle}>No photos yet</Text>
          <Text style={commonStyles.emptyBody}>Photos you take with the camera will show up here.</Text>
        </View>
      )}

      {permission === 'granted' && photos.length > 0 && (
        <FlatList
          // A list cannot change its column count in place; a new key makes a new list for the new count.
          key={columns}
          data={photos}
          extraData={selection.selected}
          numColumns={columns}
          keyExtractor={item => item.node.id}
          contentContainerStyle={[styles.grid, selection.selecting && { paddingBottom: SELECTION_BAR_CLEARANCE }]}
          columnWrapperStyle={styles.row}
          renderItem={({ item }) => (
            <SelectableThumb id={item.node.id} selection={selection} tapToSelect style={styles.thumbWrapper}>
              <Image source={{ uri: item.node.image.uri }} style={styles.thumb} />
            </SelectableThumb>
          )}
          onEndReachedThreshold={0.6}
          onEndReached={loadMore}
          refreshControl={<RefreshControl refreshing={refreshing} onRefresh={refresh} tintColor={colors.accent} />}
          ListFooterComponent={
            loadingMore ? <ActivityIndicator style={styles.footer} color={colors.accent} /> : undefined
          }
        />
      )}
    </>
  );
}

export interface HomeScreenProps {
  /** Opens the full-screen editor for a tapped photo. */
  onOpenPhoto: (photo: EditablePhoto) => void;
  /** Opens the selected plugin workspace. */
  onOpenPluginEditor: (workspace: PluginWorkspaceScreenInfo) => void;
}

type ScannedWorkspacePlugin = {
  id: string;
  name: string;
  missingDependencies: string[];
  workspaces: { id: string; title: string; description: string; entryPath: string }[];
};

const toEditablePhoto = (photo: PhotoIdentifier): EditablePhoto => ({
  uri: photo.node.image.uri,
  fileName: photo.node.image.filename,
});

/** Tapping photos selects them; the bar above the grid then adds the selected ones. */
function CameraRoll({ onAddPhotos }: { onAddPhotos: (photos: EditablePhoto[]) => Promise<void> }) {
  const { colors } = useTheme();
  const { permission, photos, loadingMore, refreshing, loadMore, refresh } = useCameraPhotos();
  const selection = useSelection();
  const addSelected = () =>
    onAddPhotos(photos.filter(photo => selection.selected.includes(photo.node.id)).map(toEditablePhoto));
  return (
    <View style={styles.albumSection}>
      <View style={styles.albumHeader}>
        <Text style={[styles.sectionTitle, { color: colors.textPrimary }]}>Camera Roll</Text>
        {permission === 'granted' && (
          <Text style={[styles.albumCount, { color: colors.textMuted }]}>
            {photos.length} photo{photos.length === 1 ? '' : 's'}
          </Text>
        )}
      </View>
      <View style={styles.albumContents}>
        <Permissions
          permission={permission}
          photos={photos}
          refreshing={refreshing}
          loadMore={loadMore}
          refresh={refresh}
          loadingMore={loadingMore}
          selection={selection}
        />
      </View>
      {selection.selecting && (
        <SelectionBar
          count={selection.selected.length}
          actionLabel="Add"
          onCancel={selection.clear}
          onPreview={selection.openPreview}
          onAction={addSelected}
        />
      )}
      <SelectionPreview
        selection={selection}
        uriOf={id => photos.find(photo => photo.node.id === id)?.node.image.uri}
        doneLabel="Add"
        onDone={addSelected}
      />
    </View>
  );
}

/**
 * Shows the saved projects. Each one is made from a photo (camera roll or file picker), is kept until it is removed,
 * and opens in the editor when tapped.
 */
function LoadedImages({ onOpenPhoto, onOpenPluginProject }: { onOpenPhoto: (photo: EditablePhoto) => void; onOpenPluginProject: (project: Project) => void }) {
  const { colors } = useTheme();
  const projects = useObservable(projects$, []);
  // Same sizing as the Camera Roll grid, inside this view's 16 padding.
  const { width } = useWindowDimensions();
  const innerWidth = width - 32;
  const columns = Math.max(MIN_COLUMNS, Math.floor((innerWidth + GAP) / (MIN_THUMB + GAP)));
  const thumbSize = (innerWidth - GAP * (columns - 1)) / columns;
  const selection = useSelection();
  const [exportOpen, setExportOpen] = useState(false);
  const { selected, selecting, clear } = selection;
  const exportProjects = useMemo(() => projects.filter(project => selected.includes(project.id)), [projects, selected]);
  const exportItems = useMemo(
    () => exportProjects.map(project => ({ id: project.id, uri: projectImageUri(project), name: project.name })),
    [exportProjects],
  );
  const saveProjectImage = async (item: ExportDialogItem, fileName: string, format: ExportFormat, quality: number) => {
    const image = await AbraPixels.readAsync(toLocalPath(item.uri));
    try {
      await writeImageToGallery(image, fileName, format, quality);
    } finally {
      release(image);
    }
  };
  const confirmRemove = () =>
    Alert.alert(
      `Delete ${selected.length} project${selected.length === 1 ? '' : 's'}?`,
      'Their edits are deleted from the app. The original photos are not touched.',
      [
        { text: 'Cancel', style: 'cancel' },
        {
          text: 'Delete',
          style: 'destructive',
          onPress: async () => {
            await removeProjects(selected);
            clear();
          },
        },
      ],
    );
  return (
    <View style={{ flex: 1, padding: 16 }}>
      {projects.length === 0 && (
        <View style={{ alignItems: 'center', justifyContent: 'center', flex: 1 }}>
          <MagicalSlogan fontSize={40} subText="No images have been loaded yet." />
        </View>
      )}
      {projects.length > 0 && (
        <FlatList
          // A list cannot change its column count in place; a new key makes a new list for the new count.
          key={columns}
          data={projects}
          extraData={selected}
          keyExtractor={project => project.id}
          renderItem={({ item: project }) => (
            <SelectableThumb
              id={project.id}
              selection={selection}
              onOpen={() => (project.plugin ? onOpenPluginProject(project) : onOpenPhoto({ uri: projectSourceUri(project), fileName: project.name, project }))}
              style={{ width: thumbSize, height: thumbSize, marginBottom: GAP }}
            >
              <Image
                source={{ uri: projectImageUri(project) }}
                style={{ width: '100%', height: '100%', backgroundColor: colors.placeholder }}
              />
            </SelectableThumb>
          )}
          numColumns={columns}
          columnWrapperStyle={{ gap: GAP }}
          contentContainerStyle={selecting ? { paddingBottom: SELECTION_BAR_CLEARANCE } : undefined}
        />
      )}
      {selecting && (
        <SelectionBar
          count={selected.length}
          actionLabel="Delete"
          destructive
          onCancel={clear}
          onPreview={selection.openPreview}
          onExport={() => setExportOpen(true)}
          onAction={confirmRemove}
        />
      )}
      <ExportDialog
        items={exportItems}
        visible={exportOpen}
        onClose={() => setExportOpen(false)}
        onSaveImage={saveProjectImage}
      />
      <SelectionPreview
        selection={selection}
        uriOf={id => {
          const project = projects.find(candidate => candidate.id === id);
          return project && projectImageUri(project);
        }}
        doneLabel="Delete"
        destructive
        onDone={confirmRemove}
      />
    </View>
  );
}

export function HomeScreen({ onOpenPhoto, onOpenPluginEditor }: HomeScreenProps) {
  const { colors } = useTheme();
  const [showCameraRoll, setShowCameraRoll] = useState(false);
  const workspacePlugins = scanPlugins(pluginsFolder()).plugins as unknown as ScannedWorkspacePlugin[];
  const workspaces = workspacePlugins
    .filter(plugin => plugin.missingDependencies.length === 0)
    .flatMap(plugin => plugin.workspaces.map(workspace => ({
      pluginId: plugin.id,
      pluginName: plugin.name,
      id: workspace.id,
      title: workspace.title,
      description: workspace.description,
      entryPath: workspace.entryPath,
    })));

  /** Opens a saved plugin project in the workspace that made it, if that plugin is still installed. */
  const openPluginProject = (project: Project) => {
    const workspace = workspaces.find(candidate => candidate.pluginId === project.plugin?.pluginId && candidate.id === project.plugin?.workspaceId);
    if (workspace) onOpenPluginEditor({ ...workspace, project });
    else Alert.alert('Plugin not installed', `${project.name} was made with a plugin that is not installed or is missing something it needs.`);
  };

  /** Makes a project of each photo, from the Camera Roll or the file chooser. One photo opens right away; several stay on the home page. */
  const addPhotos = async (photos: EditablePhoto[]) => {
    const added = (await Promise.all(photos.map(addProject))).filter(project => project !== undefined);
    setShowCameraRoll(false);
    if (photos.length === 1 && added[0]) {
      onOpenPhoto({ uri: projectSourceUri(added[0]), fileName: added[0].name, project: added[0] });
    }
  };

  return (
    <View style={[styles.container, { backgroundColor: colors.background }]}>
      {/* <View style={commonStyles.header}>
        <Text style={commonStyles.headerTitle}>Alakazam</Text>
      </View> */}

      <View style={[styles.importSection, { borderBottomColor: colors.border }]}>
        <ScrollView horizontal showsHorizontalScrollIndicator={false} contentContainerStyle={{ gap: 8 }}>
          <Card
            icon={CameraIcon}
            title="Camera Roll"
            subtitle="Select photos from your device"
            style={{ width: 280 }}
            selected={showCameraRoll}
            onPress={() => setShowCameraRoll(!showCameraRoll)}
          >
            <CardHeader icon={CameraIcon} title="Camera Roll" subtitle="Select photos from your device" />
          </Card>
          <FilePicker
            title="Choose a photo"
            subtitle="Browse this device or a connected photo app"
            variant="card"
            showPreview={false}
            style={{ width: 280 }}
            onPick={assets => addPhotos(assets.map(asset => ({ uri: asset.uri, fileName: asset.name ?? null })))}
          />
          {workspaces.map(workspace => (
            <Card
              key={`${workspace.pluginId}.${workspace.id}`}
              icon={Blocks}
              title={workspace.title}
              subtitle={workspace.description || workspace.pluginName}
              style={{ width: 280 }}
              onPress={() => onOpenPluginEditor(workspace)}
            >
              <CardHeader icon={Blocks} title={workspace.title} subtitle={workspace.description || workspace.pluginName} />
            </Card>
          ))}
        </ScrollView>
      </View>

      {!showCameraRoll && <LoadedImages onOpenPhoto={onOpenPhoto} onOpenPluginProject={openPluginProject} />}
      {showCameraRoll && <CameraRoll onAddPhotos={addPhotos} />}
    </View>
  );
}

const styles = StyleSheet.create({
  container: { flex: 1 },
  importSection: {
    paddingHorizontal: 16,
    paddingBottom: 12,
    borderBottomWidth: StyleSheet.hairlineWidth,
  },
  sectionTitle: { fontSize: 17, fontWeight: '600' },
  albumSection: { flex: 1, paddingTop: 12 },
  albumHeader: {
    flexDirection: 'row',
    alignItems: 'baseline',
    justifyContent: 'space-between',
    paddingHorizontal: 16,
    paddingBottom: 8,
  },
  albumCount: { fontSize: 13 },
  albumContents: { flex: 1 },
});
