import { getActiveProject, projects } from '@/events/projects';
import { ipcMain } from 'electron';

export interface GradientOptions {
  start: [number, number];
  end: [number, number];
  type: 'linear' | 'radial';
  opacity: number;
  colorStart: string;
  colorEnd: string;
}

ipcMain.handle('tools-gradient-apply', (event, projectId: string, options: GradientOptions) => {
  const project = projects.get(projectId);
  if (!project) {
    console.log(`Project with ID ${projectId} not found`);
    return;
  }

  try {
    // TODO: Implement gradient application using the Abra library
    // This should:
    // 1. Get the active layer or create a new gradient layer
    // 2. Apply the gradient with the given start, end, type, and opacity
    // 3. Notify the renderer of composite changes
    console.log('Applying gradient:', options);
    const project = getActiveProject();
    if (!project) {
      console.error('No active project found');
      return;
    }
    const layers = project.activeLayers();
    if (layers.length === 0) {
      console.error('No active layers in the project');
      return;
    }

    const startColor = abra.Color.fromHexString(options.colorStart);
    const endColor = abra.Color.fromHexString(options.colorEnd);
    const startPoint = [options.start[0], options.start[1]] as [number, number];
    const endPoint = [options.end[0], options.end[1]] as [number, number];
    const gradient = abra.Gradient.evenly([startColor, endColor]);
    const path = abra.GradientPath.line(startPoint, endPoint);
    gradient.setDirection(path);

    for (const layerMetadata of layers) {
      const layer = project.getLayerById(layerMetadata.id);
      if (!layer) {
        console.warn(`Layer with ID ${layerMetadata.id} not found`);
        continue;
      }
      const data = layer.imageData();
      gradient.fill(layer, data.width, data.height);
    }
  } catch (error) {
    console.error('Error applying gradient:', error);
  }
});
