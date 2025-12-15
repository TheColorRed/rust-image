import { LayerMetadata } from 'alakazam-bindings';
import { ipcMain, webContents } from 'electron';
import { filter, map, Subject, tap } from 'rxjs';
import { toFileName } from '../util/strings.js';

type Project = import('alakazam-bindings').Project;
type Layer = import('alakazam-bindings').Layer;

export interface AddLayerOptions {
  type: 'file' | 'adjustment' | 'empty' | 'group';
  name: string;
  filePath?: string;
  adjustmentType?: string;
}

export const projects: Map<string, Project> = new Map();
let activeProject: Project | null = null;

// Preview state per project
const projectPreviews: Map<string, { options: Record<string, any>; originalImageData: ImageData }> = new Map();

export function getProjectPreview(projectId: string) {
  return projectPreviews.get(projectId) || { options: {}, originalImageData: null };
}

export function setProjectPreview(projectId: string, options: Record<string, any>, originalImageData: ImageData) {
  projectPreviews.set(projectId, { options, originalImageData });
}

export function clearProjectPreview(projectId: string) {
  projectPreviews.delete(projectId);
}

export function getActiveProject(): Project | null {
  return activeProject;
}

export function setActiveProject(projectId: string | null) {
  if (projectId === null) {
    activeProject = null;
    return;
  }
  const project = projects.get(projectId) ?? null;
  activeProject = project;
}

export function openProject(filePath: string) {
  if (!filePath) return null;
  const alakazam = global.alakazam;
  const project = new alakazam.Project(toFileName(filePath), filePath);
  projects.set(project.id, project);
  setActiveProject(project.id);
  // Notify renderer processes about the new project
  webContents.getAllWebContents().forEach(wc => {
    wc.send('new-project', project.id);
  });
  return project;
}

export function closeProject(projectId: string) {
  projects.delete(projectId);
  webContents.getAllWebContents().forEach(wc => {
    wc.send('close-project', projectId);
  });
  if (projects.size === 0) {
    setActiveProject(null);
  }
}

interface NotifyParams {
  projectId: string;
  compositeChanged?: boolean;
  projectChanged?: boolean;
  layerIds?: string[];
}

const notifyRenderer = new Subject<NotifyParams>();
notifyRenderer
  .pipe(
    // Prevent rapid successive calls so we don't spam the renderer processes
    // auditTime(100),
    // Fetch the project for the given projectId
    map(params => ({ ...params, project: projects.get(params.projectId) })),
    // Only proceed if the project exists
    filter((params): params is NotifyParams & { project: Project } => typeof params.project !== 'undefined'),
    // Send IPC messages to all renderer processes notifying them of composite changes
    tap(
      ({ compositeChanged, project }) =>
        compositeChanged &&
        webContents.getAllWebContents().forEach(wc => wc.send('on-project-composite-changed', project.composite())),
    ),
    // Send IPC messages to all renderer processes notifying them of project metadata changes
    tap(
      ({ projectChanged, project }) =>
        projectChanged &&
        webContents.getAllWebContents().forEach(wc => wc.send('on-project-changed', project.metadata())),
    ),
    tap(
      ({ project, layerIds }) =>
        layerIds &&
        layerIds.length > 0 &&
        webContents.getAllWebContents().forEach(wc => {
          layerIds.forEach(layerId => {
            const layer = project.getLayerById(layerId);
            if (!layer) return;
            wc.send('on-project-layer-composite-changed', layer.metadata(), project.metadata());
          });
        }),
    ),
  )
  .subscribe();

export function onCompositeChanged(projectId: string, layerIds?: string[]) {
  notifyRenderer.next({ projectId, compositeChanged: true, projectChanged: true, layerIds });
}

export function onMetadataChanged(projectId: string) {
  notifyRenderer.next({ projectId, projectChanged: true });
}

ipcMain.handle('project-open', (_event, { filePath }) => openProject(filePath));
ipcMain.handle('project-close', (_event, { projectId }) => closeProject(projectId));

ipcMain.handle('project-get-layers', (_event, { projectId }) => {
  const project = projects.get(projectId);
  if (!project) {
    console.log(`Project with ID ${projectId} not found`);
    return [];
  }
  console.log(project);
  // return project.layers.map(layer => layer.metadata());
});

ipcMain.handle('project-get-composite', (_event, { projectId }) => {
  const project = projects.get(projectId);
  if (!project) {
    console.log(`Project with ID ${projectId} not found`);
    return null;
  }
  return project.composite();
});

ipcMain.handle('on-project-composite-changed', (_event, { projectId }) => {
  const project = projects.get(projectId);
  if (!project) {
    console.log(`Project with ID ${projectId} not found`);
    return;
  }
  onCompositeChanged(projectId);
});

ipcMain.handle('project-get-metadata', (_event, { projectId }) => {
  if (projectId) {
    const project = projects.get(projectId);
    if (!project) return null;
    return project.metadata();
  } else {
    return Array.from(projects.values()).map(project => project.metadata());
  }
});

ipcMain.handle('project-get-active', _event => {
  return getActiveProject()?.metadata() || null;
});

ipcMain.handle('project-get-layer-metadata', (_event, { projectId }) => {
  const project = projects.get(projectId);
  if (!project) {
    throw new Error(`Project with ID ${projectId} not found`);
  }
  return project.layerMetadata;
});

ipcMain.handle('project-get-layer-image', (_event, { projectId, layerId, size }) => {
  const project = projects.get(projectId);
  if (!project) {
    console.log(`Project with ID ${projectId} not found`);
    return;
  }
  const layer = project.getLayerById(layerId);
  if (!layer) {
    console.log('Layer not found:', layerId, 'in project', projectId);
    return;
  }
  // convert size to tuple if provided, use `0` for missing values
  // [0, 0] means no resizing
  // [0, height] means scale width proportionally
  // [width, 0] means scale height proportionally
  const layerSize = size.width || (size.height && [size.width || 0, size.height || 0]);
  return layer.imageData(layerSize);
});

ipcMain.handle('project-get-active-layers', (_event, { projectId }) => {
  const project = projects.get(projectId);
  if (!project) {
    console.log(`Project with ID ${projectId} not found`);
    return [];
  }
  return project.activeLayers();
});

ipcMain.handle('project-set-active-layers', (_event, { projectId, layerIds }) => {
  const project = projects.get(projectId);
  if (!project) {
    console.log(`Project with ID ${projectId} not found`);
    return;
  }
  project.setActiveLayers(layerIds);
  onMetadataChanged(projectId);
});

ipcMain.handle('project-add-layer', (_event, { projectId, options: addLayerOptions }) => {
  const options: AddLayerOptions = addLayerOptions;
  const project = projects.get(projectId);
  if (!project) {
    console.log(`Project with ID ${projectId} not found`);
    return;
  }
  let newLayer: LayerMetadata | null = null;
  if (options.type === 'file' && options.filePath) {
    newLayer = project.addLayerFromPath(options.name, options.filePath);
  } else if (options.type === 'empty') {
    newLayer = project.addEmptyLayer(options.name);
  } else if (options.type === 'adjustment' && options.adjustmentType) {
    newLayer = project.addAdjustmentLayer(options.name, options.adjustmentType);
  }
  onCompositeChanged(projectId);
  return newLayer;
});

ipcMain.handle('project-move-layer-up', (_event, { projectId, layerId }) => {
  const project = projects.get(projectId);
  if (!project) {
    console.log(`Project with ID ${projectId} not found`);
    return;
  }
  const layer = project.getLayerById(layerId);
  if (!layer) {
    console.log('Layer not found:', layerId, 'in project', projectId);
    return;
  }
  project.moveLayerUp(layer);
  onCompositeChanged(projectId);
});

ipcMain.handle('project-move-layer-down', (_event, { projectId, layerId }) => {
  const project = projects.get(projectId);
  if (!project) {
    console.log(`Project with ID ${projectId} not found`);
    return;
  }
  const layer = project.getLayerById(layerId);
  if (!layer) {
    console.log('Layer not found:', layerId, 'in project', projectId);
    return;
  }
  project.moveLayerDown(layer);
  onCompositeChanged(projectId);
});

ipcMain.handle('project-move-layer-to', (_event, { projectId, layerId, newIndex }) => {
  const project = projects.get(projectId);
  if (!project) {
    console.log(`Project with ID ${projectId} not found`);
    return;
  }
  const layer = project.getLayerById(layerId);
  if (!layer) {
    console.log('Layer not found:', layerId, 'in project', projectId);
    return;
  }
  project.moveLayerTo(layer, newIndex);
  onCompositeChanged(projectId);
});

ipcMain.handle('project-move-layer-to-bottom', (_event, { projectId, layerId }) => {
  const project = projects.get(projectId);
  if (!project) {
    console.log(`Project with ID ${projectId} not found`);
    return;
  }
  const layer = project.getLayerById(layerId);
  if (!layer) {
    console.log('Layer not found:', layerId, 'in project', projectId);
    return;
  }
  project.moveLayerToBottom(layer);
  onCompositeChanged(projectId);
});

ipcMain.handle('project-move-layer-to-top', (_event, { projectId, layerId }) => {
  const project = projects.get(projectId);
  if (!project) {
    console.log(`Project with ID ${projectId} not found`);
    return;
  }
  const layer = project.getLayerById(layerId);
  if (!layer) {
    console.log('Layer not found:', layerId, 'in project', projectId);
    return;
  }
  project.moveLayerToTop(layer);
  onCompositeChanged(projectId);
});

ipcMain.handle('project-reorder-layers', (_event, { projectId, newOrderIds }) => {
  const project = projects.get(projectId);
  if (!project) {
    console.log(`Project with ID ${projectId} not found`);
    return;
  }
  project.reorderLayers(newOrderIds);
  onCompositeChanged(projectId);
});

ipcMain.handle('project-rename-layer', (_event, { projectId, layerId, newName }) => {
  const project = projects.get(projectId);
  if (!project) {
    console.log(`Project with ID ${projectId} not found`);
    return;
  }
  const layer = project.getLayerById(layerId);
  if (!layer) {
    console.log('Layer not found:', layerId, 'in project', projectId);
    return;
  }
  layer.setName(newName);
  onCompositeChanged(projectId);
});

ipcMain.handle('project-delete-layers', (_event, { projectId, layerIds }) => {
  const project = projects.get(projectId);
  if (!project) {
    console.log(`Project with ID ${projectId} not found`);
    return;
  }
  layerIds.forEach((layerId: string) => {
    const layer = project.getLayerById(layerId);
    if (!layer) {
      console.log('Layer not found:', layerId, 'in project', projectId);
      return;
    }
    project.deleteLayer(layer);
  });
  onCompositeChanged(projectId);
});

ipcMain.handle('project-set-blend-mode', (_event, { projectId, layerIds, blendMode }) => {
  const project = projects.get(projectId);
  if (!project) {
    console.log(`Project with ID ${projectId} not found`);
    return;
  }
  layerIds.forEach((layerId: string) => {
    const layer = project.getLayerById(layerId);
    if (!layer) {
      console.log('Layer not found:', layerId, 'in project', projectId);
      return;
    }
    layer.setBlendMode(blendMode);
  });
  onCompositeChanged(projectId);
});

ipcMain.handle('project-set-opacity', (_event, { projectId, layerIds, opacity }) => {
  const project = projects.get(projectId);
  if (!project) {
    console.log(`Project with ID ${projectId} not found`);
    return;
  }
  layerIds.forEach((layerId: string) => {
    const layer = project.getLayerById(layerId);
    if (!layer) {
      console.log('Layer not found:', layerId, 'in project', projectId);
      return;
    }
    layer.setOpacity(opacity);
  });
  onCompositeChanged(projectId, layerIds);
});

ipcMain.handle('project-set-visibility', (_event, { projectId, layerIds, visible }) => {
  const project = projects.get(projectId);
  if (!project) {
    console.log(`Project with ID ${projectId} not found`);
    return;
  }
  layerIds.forEach((layerId: string) => {
    const layer = project.getLayerById(layerId);
    if (!layer) {
      console.log('Layer not found:', layerId, 'in project', projectId);
      return;
    }
    layer.setVisibility(visible);
  });
  onCompositeChanged(projectId, layerIds);
});

ipcMain.handle('project-get-layer-composite', (_event, { projectId, layerId, maxSize }) => {
  const project = projects.get(projectId);
  if (!project) {
    console.log(`Project with ID ${projectId} not found`);
    return;
  }
  const layer = project.getLayerById(layerId);
  if (!layer) {
    console.log('Layer not found:', layerId, 'in project', projectId);
    return;
  }
  return project.compositeLayer(layer, maxSize);
});
