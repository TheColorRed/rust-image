import { ipcMain } from 'electron';
import { onCompositeChanged, projects } from './projects.js';

ipcMain.handle('transform-resize-layer', (_event, { projectId, layerId, size }) => {
  const project = projects.get(projectId);
  if (!project) {
    console.log(`Project with ID ${projectId} not found.`);
    return;
  }
  const layer = project.getLayerById(layerId);
  if (!layer) {
    console.log(`Layer with ID ${layerId} not found in project ${projectId}.`);
    return;
  }
  // layer.set
  onCompositeChanged(projectId);
});

ipcMain.handle('transform-rotate-layer', (_event, { projectId, layerId, angle }) => {
  const project = projects.get(projectId);
  if (!project) {
    console.log(`Project with ID ${projectId} not found.`);
    return;
  }
  const layer = project.getLayerById(layerId);
  if (!layer) {
    console.log(`Layer with ID ${layerId} not found in project ${projectId}.`);
    return;
  }
  console.log(`Setting rotation of layer ${layerId} in project ${projectId} to ${angle} degrees.`);
  layer.setRotation(angle);
  onCompositeChanged(projectId);
});

ipcMain.handle('transform-position-layer', (_event, { projectId, layerId, position }) => {
  const project = projects.get(projectId);
  if (!project) {
    console.log(`Project with ID ${projectId} not found.`);
    return;
  }
  const layer = project.getLayerById(layerId);
  if (!layer) {
    console.log(`Layer with ID ${layerId} not found in project ${projectId}.`);
    return;
  }
  layer.setPosition(position);
  onCompositeChanged(projectId);
});

ipcMain.handle('transform-anchor-layer', (_event, { projectId, layerId, anchor }) => {
  const project = projects.get(projectId);
  if (!project) {
    console.log(`Project with ID ${projectId} not found.`);
    return;
  }
  const layer = project.getLayerById(layerId);
  if (!layer) {
    console.log(`Layer with ID ${layerId} not found in project ${projectId}.`);
    return;
  }
  layer.setAnchor(anchor);
  onCompositeChanged(projectId);
});
