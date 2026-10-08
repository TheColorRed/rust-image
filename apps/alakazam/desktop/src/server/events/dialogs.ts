import { showDialog } from '@/dialog';
import { openFileDialog, saveFileDialog } from '@/native-dialogs';
import { ipcMain } from 'electron';

ipcMain.handle('open-file-dialog', (_event, { title, properties, filters }) =>
  openFileDialog(title, properties, filters),
);
ipcMain.handle('save-file-dialog', (_event, { title, properties, filters }) =>
  saveFileDialog(title, properties, filters),
);

ipcMain.handle('show-dialog', (_event, { dialogPath, title }: { dialogPath: DialogPath; title: string }) => {
  showDialog(dialogPath, title);
});
