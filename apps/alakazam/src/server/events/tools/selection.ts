import { clearSelectionArea, setSelectionArea, setSelectionFeather } from '@/services/selection';
import { ipcMain } from 'electron';

ipcMain.handle('tools-selection-set-area', (event, projectId: string, area: [number, number][]) =>
  setSelectionArea(projectId, area),
);

ipcMain.handle('tools-selection-set-feather', (event, projectId: string, feather: number) =>
  setSelectionFeather(projectId, feather),
);

ipcMain.handle('tools-selection-clear', (event, projectId: string) => clearSelectionArea(projectId));
