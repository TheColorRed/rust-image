import { NativeModules, Platform } from 'react-native';

type MediaFileModule = {
  copyToCache(uri: string, fileName: string): Promise<string>;
  deleteFromCache(uri: string): Promise<boolean>;
};

const mediaFile = NativeModules.MediaFile as MediaFileModule | undefined;

/** Copies a Camera Roll/MediaStore URI into the app cache so native image code can read it by path. */
export async function copyMediaToCache(uri: string, fileName: string): Promise<string> {
  if (Platform.OS !== 'android' || !uri.startsWith('content://')) return uri;
  if (!mediaFile) throw new Error('The Android media-file module is unavailable. Rebuild the app and try again.');
  return mediaFile.copyToCache(uri, fileName);
}

/** Deletes a file that is in the app cache. Anything outside the cache is left alone; returns whether it was deleted. */
export async function deleteMediaFromCache(uri: string): Promise<boolean> {
  if (Platform.OS !== 'android' || !uri.startsWith('file://')) return false;
  if (!mediaFile?.deleteFromCache) throw new Error('The installed app is older than this code. Rebuild it with npm run dev:android.');
  return mediaFile.deleteFromCache(uri);
}
