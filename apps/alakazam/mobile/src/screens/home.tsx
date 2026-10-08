import { Card, CardHeader } from '@/src/components/card';
import { FilePicker } from '@/src/components/file-picker';
import {
  SELECTION_BAR_CLEARANCE,
  SelectedOverlay,
  SelectionBar,
  SelectionPreview,
  useSelection,
} from '@/src/components/selection';
import { useCameraPhotos } from '@/src/hooks/useCameraPhotos';
import { useCommonStyles } from '@/src/lib/styles';
import { useTheme } from '@/src/lib/theme';
import type { EditablePhoto } from '@/src/screens/editor';
import { projectImageUri, projectSourceUri } from '@/src/lib/project-store';
import { addProject, projects$, removeProjects } from '@/src/state/projects';
import type { PhotoIdentifier } from '@react-native-camera-roll/camera-roll';
import { CameraIcon } from 'lucide-react-native';
import { useMemo, useState } from 'react';
import {
  ActivityIndicator,
  Alert,
  FlatList,
  Image,
  Linking,
  Pressable,
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
          <Text style={commonStyles.emptyBody}>Grant access to your Camera photos to see them here.</Text>
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
            <Pressable
              onPress={() => selection.toggle(item.node.id)}
              style={styles.thumbWrapper}
            >
              <Image source={{ uri: item.node.image.uri }} style={styles.thumb} />
              {selection.selected.includes(item.node.id) && <SelectedOverlay />}
            </Pressable>
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
}

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
        <Text style={[styles.sectionTitle, { color: colors.textPrimary }]}>Camera album</Text>
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
function LoadedImages({ onOpenPhoto }: { onOpenPhoto: (photo: EditablePhoto) => void }) {
  const { colors } = useTheme();
  const projects = useObservable(projects$, []);
  // Same sizing as the Camera Roll grid, inside this view's 16 padding.
  const { width } = useWindowDimensions();
  const innerWidth = width - 32;
  const columns = Math.max(MIN_COLUMNS, Math.floor((innerWidth + GAP) / (MIN_THUMB + GAP)));
  const thumbSize = (innerWidth - GAP * (columns - 1)) / columns;
  const selection = useSelection();
  const { selected, selecting, toggle, clear } = selection;
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
          <Text style={{ color: colors.textSecondary, fontSize: 25, fontWeight: '900' }}>Abra. Kadabra. Alakazam!</Text>
          <Text style={{ color: colors.textMuted, fontSize: 16, marginTop: 8 }}>No images have been loaded yet.</Text>
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
            <Pressable
              onPress={() =>
                selecting ? toggle(project.id) : onOpenPhoto({ uri: projectSourceUri(project), fileName: project.name, project })
              }
              onLongPress={() => toggle(project.id)}
              style={{ width: thumbSize, height: thumbSize, marginBottom: GAP }}
            >
              <Image
                source={{ uri: projectImageUri(project) }}
                style={{ width: '100%', height: '100%', backgroundColor: colors.placeholder }}
              />
              {selected.includes(project.id) && <SelectedOverlay />}
            </Pressable>
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
          onAction={confirmRemove}
        />
      )}
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

export function HomeScreen({ onOpenPhoto }: HomeScreenProps) {
  const { colors } = useTheme();
  const [showCameraRoll, setShowCameraRoll] = useState(false);

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
            onPick={asset => addProject({ uri: asset.uri, fileName: asset.name ?? null })}
          />
        </ScrollView>
      </View>

      {!showCameraRoll && <LoadedImages onOpenPhoto={onOpenPhoto} />}
      {showCameraRoll && (
        <CameraRoll
          onAddPhotos={async photos => {
            await Promise.all(photos.map(addProject));
            setShowCameraRoll(false);
          }}
        />
      )}
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
