import { copyMediaToCache } from '@/src/lib/media-file';
import { type Project } from '@/src/lib/project-store';

export interface EditablePhoto {
  uri: string;
  fileName: string | null;
  /** The project this photo is the source of. Its saved edits are replayed on open and saved again on close. */
  project?: Project;
}

/** Converts a `file://` URI to the filesystem path abra reads and writes. */
export const toLocalPath = (uri: string) => decodeURIComponent(uri.replace(/^file:\/\//, ''));

/** Copies a Camera Roll `content://` photo into the app cache so abra can read it by path. */
export async function resolveLocalPhotoPath(photo: EditablePhoto): Promise<string> {
  if (!photo.uri.startsWith('content://')) return toLocalPath(photo.uri);
  const fileName = photo.fileName ?? `photo-${Date.now()}.jpg`;
  return toLocalPath(await copyMediaToCache(photo.uri, fileName));
}
