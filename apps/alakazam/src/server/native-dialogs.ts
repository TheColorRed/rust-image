import { dialog, OpenDialogOptions, SaveDialogOptions } from 'electron';

const SUPPORTED_EXTENSIONS = ['png', 'jpg', 'jpeg', 'gif', 'webp'];

export type OpenDialogProperties = OpenDialogOptions['properties'];
export type SaveDialogProperties = SaveDialogOptions['properties'];

/**
 * Opens a native file open dialog for selecting image files.
 * @returns The selected file path or null if canceled.
 */
export async function openFileDialog(
  title = 'Open Image',
  properties?: OpenDialogProperties,
  filters?: { name: string; extensions: string[] }[],
) {
  const defaultProperties = ['openFile'].concat(properties || []) as OpenDialogProperties;
  const fileFilters = filters ?? [{ name: 'Images', extensions: SUPPORTED_EXTENSIONS }];
  const result = await dialog.showOpenDialog({ properties: defaultProperties, title, filters: fileFilters });
  if (result.canceled || result.filePaths.length === 0) return null;
  return result.filePaths;
}
/**
 * Opens a native file save dialog for saving an image file.
 * @returns The selected file path or null if canceled.
 */
export async function saveFileDialog(
  title = 'Save Image',
  properties?: SaveDialogProperties,
  filters?: { name: string; extensions: string[] }[],
) {
  const defaultProperties = ['saveFile'].concat(properties ?? []) as SaveDialogProperties;
  const fileFilters = filters ?? [{ name: 'Images', extensions: SUPPORTED_EXTENSIONS }];
  const result = await dialog.showSaveDialog({ title, properties: defaultProperties, filters: fileFilters });
  if (result.canceled || !result.filePath) return null;
  return result.filePath;
}
