import type { PhotoIdentifier } from '@react-native-camera-roll/camera-roll';
import { useMemo } from 'react';
import {
    ActivityIndicator,
    Dimensions,
    FlatList,
    Image,
    Linking,
    Pressable,
    RefreshControl,
    StyleSheet,
    Text,
    TouchableOpacity,
    View,
} from 'react-native';
import { useCameraPhotos } from '@/src/hooks/useCameraPhotos';
import { useCommonStyles } from '@/src/lib/styles';
import { useTheme } from '@/src/lib/theme';
import type { EditablePhoto } from '@/src/screens/editor';

const COLUMNS = 3;
const GAP = 2;
const THUMB_SIZE = (Dimensions.get('window').width - GAP * (COLUMNS - 1)) / COLUMNS;

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
  const styles = useMemo(
    () =>
      StyleSheet.create({
        settingsButton: { marginTop: 8 },
        grid: { paddingHorizontal: 0 },
        row: { gap: GAP },
        thumbWrapper: { width: THUMB_SIZE, height: THUMB_SIZE, marginBottom: GAP },
        thumb: { width: '100%', height: '100%', backgroundColor: colors.placeholder },
        footer: { paddingVertical: 20 },
      }),
    [colors],
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
          data={photos}
          numColumns={COLUMNS}
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
        {permission === 'granted' && (
          <Text style={commonStyles.headerSubtitle}>
            {photos.length} photo{photos.length === 1 ? '' : 's'}
          </Text>
        )}
      </View>

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
  );
}

const styles = StyleSheet.create({
  container: { flex: 1 },
});
