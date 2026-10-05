import { FilePicker } from '@/src/components/file-picker';
import { useCameraPhotos } from '@/src/hooks/useCameraPhotos';
import { useCommonStyles } from '@/src/lib/styles';
import { useTheme } from '@/src/lib/theme';
import type { EditablePhoto } from '@/src/screens/editor';
import type { PhotoIdentifier } from '@react-native-camera-roll/camera-roll';
import { useMemo } from 'react';
import {
  ActivityIndicator,
  FlatList,
  Image,
  Linking,
  Pressable,
  RefreshControl,
  StyleSheet,
  Text,
  TouchableOpacity,
  useWindowDimensions,
  View,
} from 'react-native';

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
  onOpenPhoto,
}: {
  permission: 'checking' | 'denied' | 'granted';
  photos: PhotoIdentifier[];
  refreshing: boolean;
  loadMore: () => void;
  refresh: () => void;
  loadingMore: boolean;
  onOpenPhoto: (photo: EditablePhoto) => void;
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
          numColumns={columns}
          keyExtractor={item => item.node.id}
          contentContainerStyle={styles.grid}
          columnWrapperStyle={styles.row}
          renderItem={({ item }) => (
            <Pressable
              onPress={() => onOpenPhoto({ uri: item.node.image.uri, fileName: item.node.image.filename })}
              style={styles.thumbWrapper}
            >
              <Image source={{ uri: item.node.image.uri }} style={styles.thumb} />
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

export default function HomeScreen({ onOpenPhoto }: HomeScreenProps) {
  const { colors } = useTheme();
  const commonStyles = useCommonStyles();
  const { permission, photos, loadingMore, refreshing, loadMore, refresh } = useCameraPhotos();

  return (
    <View style={[styles.container, { backgroundColor: colors.background }]}>
      <View style={commonStyles.header}>
        <Text style={commonStyles.headerTitle}>Alakazam</Text>
      </View>

      <View style={[styles.importSection, { borderBottomColor: colors.border }]}>
        <Text style={[styles.sectionTitle, { color: colors.textPrimary }]}>Open an image</Text>
        <FilePicker
          title="Choose a photo"
          subtitle="Browse this device or a connected photo app"
          variant="card"
          showPreview={false}
          onPick={asset => onOpenPhoto({ uri: asset.uri, fileName: asset.name ?? null })}
        />
      </View>

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
            onOpenPhoto={onOpenPhoto}
          />
        </View>
      </View>
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
