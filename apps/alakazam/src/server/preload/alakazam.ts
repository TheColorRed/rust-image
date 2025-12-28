import { type AddLayerOptions } from '@/events/projects';
import { OpenDialogProperties, SaveDialogProperties } from '@/native-dialogs';
import { LayerMetadata, ProjectMetadata } from '@alakazam/abra';
import { contextBridge, ipcRenderer } from 'electron';
import { auditTime, Subject } from 'rxjs';

export type DialogConsoleMessage = {
  message: string;
  level: number;
  line: number;
  sourceId: string;
};

export type InstantAdjustmentType = 'autoColor' | 'autoTone' | 'invert' | 'grayscale';

export type Anchor =
  | 'top-left'
  | 'top-center'
  | 'top-right'
  | 'center-left'
  | 'center'
  | 'center-right'
  | 'bottom-left'
  | 'bottom-center'
  | 'bottom-right'
  | null;

export interface AlakazamApi {
  /** Opens a native file dialog and returns the selected file path, or null if cancelled. */
  openFileDialog: (
    title: string,
    settings?: OpenDialogProperties,
    filters?: { name: string; extensions: string[] }[],
  ) => Promise<string[] | null>;
  saveFileDialog: (
    title: string,
    settings?: SaveDialogProperties,
    filters?: { name: string; extensions: string[] }[],
  ) => Promise<string | null>;
  onDialogConsoleMessage: (callback: (message: DialogConsoleMessage) => void) => () => void;
  onNewProject: (callback: (projectId: string) => void) => () => void;
  onCloseProject: (callback: (projectId: string) => void) => () => void;
  onWindowLostFocus: (callback: () => void) => () => void;
  updateDialogWindowSize: () => void;

  // Title bar APIs
  showMenu: (menuLabel: string) => Promise<void>;
  minimizeWindow: () => Promise<void>;
  maximizeWindow: () => Promise<void>;
  closeWindow: () => Promise<void>;
  isMaximized: () => Promise<boolean>;

  // Dialog APIs
  showDialog: (dialogPath: DialogPath, title: string) => Promise<void>;
  developer: {
    /** Checks if the app is running in development mode. */
    isDev: () => Promise<boolean>;
    /** Toggles the developer tools for the current window. */
    toggleDevTools: () => Promise<void>;
  };
  layers: {
    /**
     * Gets metadata for all layers in the specified project.
     * @param projectId The project ID to retrieve layers from.
     * @returns An array of LayerMetadata objects.
     */
    getLayerMetadata: (projectId: string) => Promise<LayerMetadata[]>;
    /**
     * Gets the image data for a specific layer in a project.
     * @param projectId The project ID.
     * @param layerId The layer ID.
     * @param size Optional size to scale the image to. If only width or height is provided, the other dimension will be scaled proportionally.
     * @returns The ImageData of the specified layer.
     */
    getLayerImage: (
      projectId: string,
      layerId: string,
      size?: { width?: number; height?: number },
    ) => Promise<ImageData>;
    /**
     * Gets the composite image data for a specific layer in a project.
     * @param projectId The project ID.
     * @param layerId The layer ID.
     * @param maxSize The maximum size (width or height) for the returned image. The image will be scaled proportionally.
     * @returns The ImageData of the layer's composite.
     */
    getLayerComposite: (projectId: string, layerId: string, maxSize: number) => Promise<ImageData>;
  };

  projects: {
    /**
     * Opens a project from the given file path.
     * @param filePath The path to the project file to open.
     * @returns A unique identifier for the opened project.
     */
    openProject: (filePath: string) => Promise<{ projectId: string; filePath: string }>;
    /**
     * Exports the active project to the specified file path.
     * @param filePath The path to export the active project to.
     */
    exportActiveProject: (filePath: string) => Promise<void>;
    /**
     * Adds a new layer to the specified project.
     * @param projectId The ID of the project to add the layer to.
     * @param options The options for the new layer.
     * @returns The metadata of the newly added layer.
     */
    addLayer: (projectId: string, options: AddLayerOptions) => Promise<LayerMetadata>;
    pasteImageFromClipboard: (projectId: string) => Promise<void>;
    /**
     * Moves the specified layer up by one position in the layer stack.
     * @param projectId The ID of the project containing the layer.
     * @param layerId The ID of the layer to move.
     * @returns The updated LayerMetadata after moving.
     */
    moveLayerUp: (projectId: string, layerId: string) => Promise<LayerMetadata>;
    /**
     * Moves the specified layer down by one position in the layer stack.
     * @param projectId The ID of the project containing the layer.
     * @param layerId The ID of the layer to move.
     * @returns The updated LayerMetadata after moving.
     */
    moveLayerDown: (projectId: string, layerId: string) => Promise<LayerMetadata>;
    /**
     * Moves the specified layer to a new position in the layer stack.
     * @param projectId The ID of the project containing the layer.
     * @param layerId The ID of the layer to move.
     * @param newIndex The new index position for the layer.
     * @returns The updated LayerMetadata after moving.
     */
    moveLayerTo: (projectId: string, layerId: string, newIndex: number) => Promise<LayerMetadata>;
    /**
     * Moves the specified layer to the bottom of the layer stack.
     * @param projectId The ID of the project containing the layer.
     * @param layerId The ID of the layer to move.
     * @returns The updated LayerMetadata after moving.
     */
    moveLayerToBottom: (projectId: string, layerId: string) => Promise<LayerMetadata>;
    /**
     * Moves the specified layer to the top of the layer stack.
     * @param projectId The ID of the project containing the layer.
     * @param layerId The ID of the layer to move.
     * @returns The updated LayerMetadata after moving.
     */
    moveLayerToTop: (projectId: string, layerId: string) => Promise<LayerMetadata>;
    /**
     * Reorders layers in the specified project according to the provided array of layer IDs.
     * @param projectId The ID of the project containing the layers.
     * @param newOrderIds An array of layer IDs representing the new order.
     */
    reorderLayers: (projectId: string, newOrderIds: string[]) => Promise<void>;
    /**
     * Renames the specified layer in a project.
     * @param projectId The ID of the project containing the layer.
     * @param layerId The ID of the layer to rename.
     * @param newName The new name for the layer.
     * @returns The updated LayerMetadata after renaming.
     */
    renameLayer: (projectId: string, layerId: string, newName: string) => Promise<LayerMetadata>;
    /**
     * Closes the specified project.
     * @param projectId The ID of the project to close.
     */
    closeProject: (projectId: string) => Promise<void>;
    /**
     * Gets the composite image of the specified project.
     * @param projectId The ID of the project to retrieve.
     * @returns The project object.
     */
    getComposite: (projectId: string) => Promise<ImageData>;
    /**
     * Registers a listener for composite image changes.
     * @returns A promise that resolves to the updated ImageData when the composite changes.
     */
    onCompositeChanged: (callback: (composite: ImageData) => void) => () => void;
    /**
     * Registers a listener for layer composite changes.
     * @returns A function to unsubscribe the listener.
     */
    onLayerCompositeChanged: (callback: (layer: LayerMetadata, project: ProjectMetadata) => void) => () => void;
    /**
     * Registers a listener for project metadata changes.
     * @param callback The callback function to invoke when the project changes.
     * @returns A function to unsubscribe the listener.
     */
    onProjectChanged: (callback: (composite: ProjectMetadata) => void) => () => void;
    /**
     * Gets metadata for projects, optionally filtered by a specific project ID.
     * @param projectId An optional project ID to filter metadata.
     * @returns An array of project metadata or a single project metadata object, or null if not found.
     */
    getProjectMetadata: <T extends string | undefined>(
      projectId?: T,
    ) => T extends string ? Promise<ProjectMetadata | null> : Promise<ProjectMetadata[]>;
    /**
     * Gets the currently active project.
     * @returns The active Project object or null if no project is active.
     */
    getActiveProjectMetadata: () => Promise<ProjectMetadata | null>;
    /**
     * Gets the active layers of the specified project.
     * @param projectId The ID of the project.
     * @returns An array of LayerMetadata objects representing the active layers.
     */
    getActiveLayers: (projectId: string) => Promise<LayerMetadata[]>;
    /**
     * Sets the active layers for the specified project.
     * @param projectId The ID of the project.
     * @param layerIds An array of layer IDs to set as active.
     */
    setActiveLayers: (projectId: string, layerIds: string[]) => Promise<void>;
    /**
     * Deletes the specified layers from the project.
     * @param projectId The ID of the project.
     * @param layerIds An array of layer IDs to delete.
     */
    deleteLayers: (projectId: string, layerIds: string[]) => Promise<void>;
    /**
     * Sets the blend mode for the specified layers in a project.
     * @param projectId The ID of the project.
     * @param layerIds The IDs of the layers to delete.
     * @param blendMode The blend mode to set.
     */
    setBlendMode: (projectId: string, layerIds: string[], blendMode: string) => Promise<void>;
    /**
     * Sets the opacity for the specified layers in a project.
     * @param projectId The ID of the project.
     * @param layerIds The IDs of the layers.
     * @param opacity The opacity to set (0-1).
     */
    setOpacity: (projectId: string, layerIds: string[], opacity: number) => Promise<void>;
    /** Sets the visibility for the specified layers in a project.
     * @param projectId The ID of the project.
     * @param layerIds The IDs of the layers.
     * @param visible The visibility to set (true for visible, false for hidden).
     */
    setVisibility: (projectId: string, layerIds: string[], visible: boolean) => Promise<void>;
  };

  adjustments: {
    /**
     * Applies an adjustment of the specified type with optional parameters.
     * @param type The type of adjustment to apply (e.g., 'brightness-contrast').
     * @param options Optional parameters for the adjustment.
     */
    previewAdjustment: (type: string, options?: any) => Promise<void>;
    /**
     * Applies the previewed adjustment.
     * @param type The type of adjustment to apply.
     */
    applyAdjustment: (type: string) => Promise<void>;
    /**
     * Cancels the previewed adjustment, resetting to original.
     * @param type The type of adjustment to cancel.
     */
    cancelAdjustment: (type: string) => Promise<void>;
    /**
     * Applies an instant adjustment of the specified type.
     * @param adjustmentType The type of instant adjustment to apply.
     */
    applyInstantAdjustment: (adjustmentType: InstantAdjustmentType) => Promise<void>;
  };

  imageData: {
    /**
     * Gets pixel data for the specified layers in a project within a defined area.
     * @param projectId The ID of the project.
     * @param layerIds The IDs of the layers to get pixel data from.
     * @param area The area within the layers to get pixel data from, defined as [x, y, width, height].
     */
    getPixels: (projectId: string, area: [number, number, number, number]) => Promise<ImageData>;
  };

  transform: {
    resizeLayer: (projectId: string, layerId: string, size: { width?: number; height?: number }) => Promise<void>;
    rotateLayer: (projectId: string, layerId: string, angle: number) => Promise<void>;
    positionLayer: (projectId: string, layerId: string, position: { x?: number; y?: number }) => Promise<void>;
    anchorLayer: (projectId: string, layerId: string, anchor: Anchor) => Promise<void>;
  };
}

// Throttle the adjustment preview to avoid overwhelming Abra library.
const previewAdjustment = new Subject<{ type: string; options?: any }>();
previewAdjustment
  .pipe(auditTime(100))
  .subscribe(({ type, options }) => ipcRenderer.invoke('preview-adjustment', { type, options }));

contextBridge.exposeInMainWorld('alakazam', {
  openFileDialog: (title, properties, filters) =>
    ipcRenderer.invoke('open-file-dialog', { title, properties, filters }),
  saveFileDialog: (title, properties, filters) =>
    ipcRenderer.invoke('save-file-dialog', { title, properties, filters }),
  updateDialogWindowSize: () => ipcRenderer.invoke('update-dialog-window-size'),
  // Title bar APIs
  showMenu: (menuLabel: string) => ipcRenderer.invoke('show-menu', menuLabel),
  minimizeWindow: () => ipcRenderer.invoke('minimize-window'),
  maximizeWindow: () => ipcRenderer.invoke('maximize-window'),
  closeWindow: () => ipcRenderer.invoke('close-window'),
  isMaximized: () => ipcRenderer.invoke('is-maximized'),
  onWindowLostFocus: (callback: () => void) => {
    const cb = () => callback();
    ipcRenderer.on('window-lost-focus', cb);
    return () => ipcRenderer.removeListener('window-lost-focus', cb);
  },
  onDialogConsoleMessage: (callback: (message: DialogConsoleMessage) => void) => {
    const cb = (_event: Electron.IpcRendererEvent, message: DialogConsoleMessage) => callback(message);
    ipcRenderer.on('dialog-console-message', cb);
    return () => ipcRenderer.removeListener('dialog-console-message', cb);
  },
  onNewProject: (callback: (projectId: string) => void) => {
    const cb = (_event: Electron.IpcRendererEvent, projectId: string) => callback(projectId);
    ipcRenderer.on('new-project', cb);
    return () => ipcRenderer.removeListener('new-project', cb);
  },
  onCloseProject: (callback: (projectId: string) => void) => {
    const cb = (_event: Electron.IpcRendererEvent, projectId: string) => callback(projectId);
    ipcRenderer.on('close-project', cb);
    return () => ipcRenderer.removeListener('close-project', cb);
  },

  showDialog: (dialogPath, title) => ipcRenderer.invoke('show-dialog', { dialogPath, title }),
  layers: {
    getLayerMetadata: projectId => ipcRenderer.invoke('project-get-layer-metadata', { projectId }),
    getLayerImage: (projectId, layerId, size) =>
      ipcRenderer.invoke('project-get-layer-image', { projectId, layerId, size }),
    getLayerComposite: (projectId, layerId, maxSize) =>
      ipcRenderer.invoke('project-get-layer-composite', { projectId, layerId, maxSize }),
  },
  developer: {
    isDev: () => ipcRenderer.invoke('is-dev'),
    toggleDevTools: () => ipcRenderer.invoke('toggle-dev-tools'),
  },
  projects: {
    openProject: filePath => ipcRenderer.invoke('project-open', { filePath }),
    exportActiveProject: filePath => ipcRenderer.invoke('project-export-active', { filePath }),
    addLayer: (projectId, options) => ipcRenderer.invoke('project-add-layer', { projectId, options }),
    pasteImageFromClipboard: projectId => ipcRenderer.invoke('project-paste-image-from-clipboard', { projectId }),
    moveLayerUp: (projectId, layerId) => ipcRenderer.invoke('project-move-layer-up', { projectId, layerId }),
    moveLayerDown: (projectId, layerId) => ipcRenderer.invoke('project-move-layer-down', { projectId, layerId }),
    moveLayerTo: (projectId, layerId, newIndex) =>
      ipcRenderer.invoke('project-move-layer-to', { projectId, layerId, newIndex }),
    moveLayerToBottom: (projectId, layerId) =>
      ipcRenderer.invoke('project-move-layer-to-bottom', { projectId, layerId }),
    moveLayerToTop: (projectId, layerId) => ipcRenderer.invoke('project-move-layer-to-top', { projectId, layerId }),
    reorderLayers: (projectId, newOrderIds) => ipcRenderer.invoke('project-reorder-layers', { projectId, newOrderIds }),
    renameLayer: (projectId, layerId, newName) =>
      ipcRenderer.invoke('project-rename-layer', { projectId, layerId, newName }),
    closeProject: projectId => ipcRenderer.invoke('project-close', { projectId }),
    getComposite: projectId => ipcRenderer.invoke('project-get-composite', { projectId }),
    onCompositeChanged: callback => {
      const cb = (_event: Electron.IpcRendererEvent, composite: ImageData) => callback(composite);
      ipcRenderer.on('on-project-composite-changed', cb);
      return () => ipcRenderer.removeListener('on-project-composite-changed', cb);
    },
    onLayerCompositeChanged: callback => {
      const cb = (_event: Electron.IpcRendererEvent, layer: LayerMetadata, project: ProjectMetadata) =>
        callback(layer, project);
      ipcRenderer.on('on-project-layer-composite-changed', cb);
      return () => ipcRenderer.removeListener('on-project-layer-composite-changed', cb);
    },
    onProjectChanged: callback => {
      const cb = (_event: Electron.IpcRendererEvent, project: ProjectMetadata) => callback(project);
      ipcRenderer.on('on-project-changed', cb);
      return () => ipcRenderer.removeListener('on-project-changed', cb);
    },
    getProjectMetadata: projectId => ipcRenderer.invoke('project-get-metadata', { projectId }),
    getActiveProjectMetadata: () => ipcRenderer.invoke('project-get-active'),
    getActiveLayers: projectId => ipcRenderer.invoke('project-get-active-layers', { projectId }),
    setActiveLayers: (projectId, layerIds) => ipcRenderer.invoke('project-set-active-layers', { projectId, layerIds }),
    deleteLayers: (projectId, layerIds) => ipcRenderer.invoke('project-delete-layers', { projectId, layerIds }),
    setBlendMode: (projectId, layerIds, blendMode) =>
      ipcRenderer.invoke('project-set-blend-mode', { projectId, layerIds, blendMode }),
    setOpacity: (projectId, layerIds, opacity) =>
      ipcRenderer.invoke('project-set-opacity', { projectId, layerIds, opacity }),
    setVisibility: (projectId, layerIds, visible) =>
      ipcRenderer.invoke('project-set-visibility', { projectId, layerIds, visible }),
  },
  adjustments: {
    previewAdjustment: (type: string, options?: any) => {
      previewAdjustment.next({ type, options });
      return Promise.resolve();
    },
    applyAdjustment: (type: string) => ipcRenderer.invoke('apply-adjustment', { type }),
    cancelAdjustment: (type: string) => ipcRenderer.invoke('cancel-adjustment', { type }),
    applyInstantAdjustment: (adjustmentType: 'autoColor' | 'autoTone' | 'invert' | 'grayscale') =>
      ipcRenderer.invoke('apply-instant-adjustment', { adjustmentType }),
  },
  imageData: {
    getPixels: (projectId, area: [number, number, number, number]) =>
      ipcRenderer.invoke('image-data-get-pixels', { projectId, area }),
  },
  transform: {
    resizeLayer: (projectId: string, layerId: string, size: { width: number; height: number }) =>
      ipcRenderer.invoke('transform-resize-layer', { projectId, layerId, size }),
    rotateLayer: (projectId: string, layerId: string, angle: number) =>
      ipcRenderer.invoke('transform-rotate-layer', { projectId, layerId, angle }),
    positionLayer: (projectId: string, layerId: string, position: { x: number; y: number }) =>
      ipcRenderer.invoke('transform-position-layer', { projectId, layerId, position }),
    anchorLayer: (projectId: string, layerId: string, anchor: Anchor) =>
      ipcRenderer.invoke('transform-anchor-layer', { projectId, layerId, anchor }),
  },
} as AlakazamApi);
