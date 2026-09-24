import { getActiveProject, projects } from '@/events/projects';
import { type Layer } from '@alakazam/core';
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

ipcMain.handle('image-data-sample-color', async (_event, { x, y, width, height, style, layerId }) => {
  const project = getActiveProject();
  if (!project) {
    console.error(`No active project found.`);
    return null;
  }

  let layer: Layer | null = null;
  if (layerId) layer = project.getLayerById(layerId);

  const color = abra.sampleColor(project, x, y, width, height, style, layer);
  return color.toHexString();
});
