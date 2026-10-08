import { CameraRoll, type PhotoIdentifier } from '@react-native-camera-roll/camera-roll';
import { useCallback, useEffect, useRef, useState } from 'react';
import { PermissionsAndroid, Platform } from 'react-native';

const PAGE_SIZE = 60;

export type PermissionState = 'checking' | 'granted' | 'denied';

async function requestReadPhotosPermission(): Promise<boolean> {
  // API 33+ split READ_EXTERNAL_STORAGE into per-media-type permissions.
  const permission =
    Platform.OS === 'android' && Platform.Version >= 33
      ? PermissionsAndroid.PERMISSIONS.READ_MEDIA_IMAGES
      : PermissionsAndroid.PERMISSIONS.READ_EXTERNAL_STORAGE;
  const result = await PermissionsAndroid.request(permission);
  return result === PermissionsAndroid.RESULTS.GRANTED;
}

/**
 * Paginated access to the device's Camera photo album (DCIM/Camera), requesting the read-media
 * permission on first use.
 */
export function useCameraPhotos() {
  const [permission, setPermission] = useState<PermissionState>('checking');
  const [photos, setPhotos] = useState<PhotoIdentifier[]>([]);
  const [loadingMore, setLoadingMore] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const cursor = useRef<string | undefined>(undefined);
  const hasNextPage = useRef(true);

  const loadPage = useCallback(async (reset: boolean) => {
    const page = await CameraRoll.getPhotos({
      first: PAGE_SIZE,
      after: reset ? undefined : cursor.current,
      assetType: 'Photos',
      groupTypes: 'All',
      groupName: 'Camera',
      // The edit screen needs a real filename to copy the picked photo to a local path.
      include: ['filename'],
    });
    cursor.current = page.page_info.end_cursor;
    hasNextPage.current = page.page_info.has_next_page;
    setPhotos((previous) => (reset ? page.edges : [...previous, ...page.edges]));
  }, []);

  useEffect(() => {
    let cancelled = false;
    requestReadPhotosPermission().then(async (granted) => {
      if (cancelled) return;
      setPermission(granted ? 'granted' : 'denied');
      if (granted) await loadPage(true);
    });
    return () => {
      cancelled = true;
    };
  }, [loadPage]);

  const loadMore = useCallback(async () => {
    if (loadingMore || !hasNextPage.current || permission !== 'granted') return;
    setLoadingMore(true);
    try {
      await loadPage(false);
    } finally {
      setLoadingMore(false);
    }
  }, [loadPage, loadingMore, permission]);

  const refresh = useCallback(async () => {
    setRefreshing(true);
    try {
      await loadPage(true);
    } finally {
      setRefreshing(false);
    }
  }, [loadPage]);

  return { permission, photos, loadingMore, refreshing, loadMore, refresh };
}
