import { ipcMain } from 'electron';
import { showDialog } from '../dialog.js';
import { openFileDialog as dialog, saveFileDialog } from '../native-dialogs.js';

ipcMain.handle('open-file-dialog', (_event, properties) => dialog(properties));
ipcMain.handle('save-file-dialog', () => saveFileDialog());

ipcMain.handle('show-dialog', (_event, { dialogPath, title }: { dialogPath: DialogPath; title: string }) => {
  showDialog(dialogPath, title);
});
