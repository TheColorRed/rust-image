import { NativeModules, Platform, Share } from 'react-native';

type MediaFileModule = {
  copyToCache(uri: string, fileName: string): Promise<string>;
  deleteFromCache(uri: string): Promise<boolean>;
  newCacheFile(fileName: string): Promise<string>;
  shareImage(uri: string): Promise<void>;
};

const mediaFile = NativeModules.MediaFile as MediaFileModule | undefined;

/** Copies a Camera Roll/MediaStore URI into the app cache so native image code can read it by path. */
export async function copyMediaToCache(uri: string, fileName: string): Promise<string> {
  if (Platform.OS !== 'android' || !uri.startsWith('content://')) return uri;
  if (!mediaFile) throw new Error('The Android media-file module is unavailable. Rebuild the app and try again.');
  return mediaFile.copyToCache(uri, fileName);
}

/** Makes a new empty folder in the app cache and returns the `file://` URI of a file with this name inside it. Android only. */
export async function cacheFileUri(fileName: string): Promise<string> {
  if (!mediaFile?.newCacheFile) throw new Error('The installed app is older than this code. Rebuild it with npm run dev:android.');
  return mediaFile.newCacheFile(fileName);
}

/** Opens the phone's share sheet with an image file (a `file://` URI in the app cache). Resolves when the sheet is opened. */
export async function shareImageFile(uri: string): Promise<void> {
  if (Platform.OS === 'ios') {
    await Share.share({ url: uri });
    return;
  }
  if (!mediaFile?.shareImage) throw new Error('The installed app is older than this code. Rebuild it with npm run dev:android.');
  await mediaFile.shareImage(uri);
}

/** Deletes a file that is in the app cache. Anything outside the cache is left alone; returns whether it was deleted. */
export async function deleteMediaFromCache(uri: string): Promise<boolean> {
  if (Platform.OS !== 'android' || !uri.startsWith('file://')) return false;
  if (!mediaFile?.deleteFromCache) throw new Error('The installed app is older than this code. Rebuild it with npm run dev:android.');
  return mediaFile.deleteFromCache(uri);
}
