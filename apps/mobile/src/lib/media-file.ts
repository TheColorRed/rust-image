import { NativeModules, Platform } from 'react-native';

type MediaFileModule = {
  copyToCache(uri: string, fileName: string): Promise<string>;
};

const mediaFile = NativeModules.MediaFile as MediaFileModule | undefined;

/** Copies a Camera Roll/MediaStore URI into the app cache so native image code can read it by path. */
export async function copyMediaToCache(uri: string, fileName: string): Promise<string> {
  if (Platform.OS !== 'android' || !uri.startsWith('content://')) return uri;
  if (!mediaFile) throw new Error('The Android media-file module is unavailable. Rebuild the app and try again.');
  return mediaFile.copyToCache(uri, fileName);
}
