import { projects } from '@/events/projects';
import { ipcMain } from 'electron';

ipcMain.handle('image-data-get-pixels', async (_event, { projectId, area }) => {
  const project = projects.get(projectId);
  if (!project) {
    console.error(`Project with ID ${projectId} not found.`);
    return null;
  }

  const abraArea = abra.Area.rect([area[0], area[1]], [area[2], area[3]]);

  return abra.getPixels(project, abraArea);
});
