import { dialog, OpenDialogOptions } from 'electron';

const SUPPORTED_EXTENSIONS = ['png', 'jpg', 'jpeg', 'gif', 'webp'];

type OpenDialogProperties = OpenDialogOptions['properties'];

/**
 * Opens a native file open dialog for selecting image files.
 * @returns The selected file path or null if canceled.
 */
export async function openFileDialog(
  properties?: OpenDialogProperties,
  filters?: { name: string; extensions: string[] }[],
) {
  const defaultProperties = ['openFile'].concat(properties || []) as OpenDialogProperties;
  const result = await dialog.showOpenDialog({
    properties: defaultProperties,
    title: 'Open Image',
    filters: [{ name: 'Images', extensions: SUPPORTED_EXTENSIONS }],
  });
  if (result.canceled || result.filePaths.length === 0) return null;
  return result.filePaths;
}
/**
 * Opens a native file save dialog for saving an image file.
 * @returns The selected file path or null if canceled.
 */
export async function saveFileDialog() {
  const result = await dialog.showSaveDialog({
    title: 'Save Image',
    filters: [{ name: 'Images', extensions: SUPPORTED_EXTENSIONS }],
  });
  if (result.canceled || !result.filePath) return null;
  return result.filePath;
}
