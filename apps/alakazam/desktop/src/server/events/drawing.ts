import { ipcMain } from 'electron';

ipcMain.handle('get-transparent-pattern', async (_event, { width, height, checkerSize }) => {
  // return global.alakazam.transparentPattern(width, height, checkerSize);
});
