import { addHistoryEntry } from '@/events/history';
import {
  clearProjectPreview,
  getActiveProject,
  getProjectPreview,
  onCompositeChanged,
  setProjectPreview,
} from '@/events/projects';
import { getSelectionArea, getSelectionFeather } from '@/services/selection';
import { LensBlurOptions } from '@alakazam/abra';
import { ipcMain } from 'electron';

export type AdjustmentTypes = 'autoColor' | 'autoTone' | 'invert' | 'grayscale';
/**
 * Applies an instant adjustment to the active project's background layer.
 * This adjustment type is from items that don't require a dialog and don't have adjustable parameters.
 * @param type Adjustment type
 * @param options Adjustment options
 */
export function applyInstantAdjustment<T>(type: AdjustmentTypes, options?: T) {
  const project = getActiveProject();
  if (!project) return;
  const layer = project.getLayerByName('Background');
  if (!layer) return;

  let compositeChanged = true;
  switch (type) {
    case 'autoColor':
      abra.autoColor(layer);
      break;
    case 'autoTone':
      abra.autoTone(layer);
      break;
    case 'invert':
      abra.invert(layer);
      break;
    case 'grayscale':
      abra.grayscale(layer);
      break;
    default:
      compositeChanged = false;
      break;
  }

  if (compositeChanged) onCompositeChanged(project.id);
}
/**
 * Applies the adjustment permanently to the layer.
 * @param options Adjustment options
 */
export function applyAdjustment<T>(options?: T) {
  const project = getActiveProject();
  if (!project) return;

  const layersMetadata = project.activeLayers();
  for (const layerMetadata of layersMetadata) {
    const layer = project.getLayerById(layerMetadata.id);
    if (!layer) continue;
    const data = layer.imageData();
    const entry = new global.alakazamHistory.AbraHistoryEntry('Applied adjustment', data);
    addHistoryEntry(project.id, entry);
  }

  // Clear preview, the layer is already adjusted
  clearProjectPreview(project.id);
  onCompositeChanged(project.id);
}
/**
 * Cancels the adjustment, resetting to original before adjustment parameters were applied from adjustment dialog.
 * @param options Adjustment options
 */
export function cancelAdjustment<T>(options?: T) {
  const project = getActiveProject();
  if (!project) return;
  const layersMetadata = project.activeLayers();
  for (const layerMetadata of layersMetadata) {
    const layer = project.getLayerById(layerMetadata.id);
    if (!layer) continue;
    if (!layer) return;

    const preview = getProjectPreview(project.id);
    if (preview.originalImageData) {
      // Reset to original
      layer.setImageData(preview.originalImageData);
      layer.markDirty();
    }
  }
  // Clear preview
  clearProjectPreview(project.id);
  onCompositeChanged(project.id);
}

export function previewAdjustment<T>(type: DialogFeatureType, options?: T) {
  const project = getActiveProject();
  if (!project) return;

  // Get all active layers to apply adjustments to
  const layersMetadata = project.activeLayers();
  for (const layerMetadata of layersMetadata) {
    const layer = project.getLayerById(layerMetadata.id);
    if (!layer) continue;

    // Update preview options
    const preview = getProjectPreview(project.id);
    const newOptions: Record<string, unknown> = { ...preview.options, ...(options || {}) };

    // If not saved, save original
    if (!preview.originalImageData) setProjectPreview(project.id, newOptions, layer.imageData());
    else setProjectPreview(project.id, newOptions, preview.originalImageData);

    // Get updated preview
    const currentPreview = getProjectPreview(project.id);

    if (!currentPreview.originalImageData) continue;
    // Reset to original and apply adjustments
    layer.setImageData(currentPreview.originalImageData);
    layer.markDirty();

    // Setup the adjustment options
    // The selection is in global space so we need to convert to local space
    // since the operation is performed on the image
    const adjustmentOptions = new abra.ApplyOptions();
    const selection = getSelectionArea(project.id);
    const feather = getSelectionFeather(project.id);
    const { x, y } = layer.position();
    const adjustedSelection = selection.map(([sx, sy]) => [sx - x, sy - y] as [number, number]);
    const area = abra.Area.fromPoints(adjustedSelection);
    area.setFeather(feather);
    adjustmentOptions.setArea([area]);

    // Apply adjustments
    switch (type) {
      case 'brightness-contrast':
        if (typeof newOptions.brightness !== 'number' || typeof newOptions.contrast !== 'number') break;
        abra.brightness(layer, newOptions.brightness, adjustmentOptions);
        abra.contrast(layer, newOptions.contrast, adjustmentOptions);
        break;
      case 'exposure':
        if (
          typeof newOptions.exposure !== 'number' ||
          typeof newOptions.offset !== 'number' ||
          typeof newOptions.gamma !== 'number'
        )
          break;
        abra.exposure(layer, newOptions.exposure, newOptions.offset, newOptions.gamma, adjustmentOptions);
        break;
      case 'vibrance':
        if (typeof newOptions.vibrance !== 'number' || typeof newOptions.saturation !== 'number') break;
        abra.vibrance(layer, newOptions.vibrance, newOptions.saturation, adjustmentOptions);
        break;
      // Blur adjustments
      case 'box-blur':
        if (typeof newOptions.radius !== 'number') break;
        abra.boxBlur(layer, newOptions.radius, adjustmentOptions);
        break;
      case 'gaussian-blur':
        if (typeof newOptions.radius !== 'number') break;
        abra.gaussianBlur(layer, newOptions.radius, adjustmentOptions);
        break;
      case 'lens-blur':
        if (typeof newOptions === 'undefined') break;
        abra.lensBlur(layer, newOptions as unknown as LensBlurOptions, adjustmentOptions);
        break;
      case 'motion-blur':
        if (typeof newOptions.angle !== 'number' || typeof newOptions.distance !== 'number') break;
        abra.motionBlur(layer, newOptions.angle, newOptions.distance, adjustmentOptions);
        break;
      case 'surface-blur':
        if (typeof newOptions.radius !== 'number' || typeof newOptions.threshold !== 'number') break;
        abra.surfaceBlur(layer, newOptions.radius, newOptions.threshold, adjustmentOptions);
        break;
      // Distort adjustments
      case 'pinch':
        if (typeof newOptions.amount !== 'number') break;
        // convert number to to -1.0 to 1.0 range
        const pinchAmount = Math.max(-100, Math.min(100, newOptions.amount as number)) / 100;
        abra.pinch(layer, pinchAmount, adjustmentOptions);
        break;
      case 'ripple':
        if (typeof newOptions.amount !== 'number' || typeof newOptions.size !== 'string') break;
        const rippleAmount = Math.max(-100, Math.min(100, newOptions.amount as number)) / 100;
        abra.ripple(
          layer,
          rippleAmount,
          newOptions.size as 'small' | 'medium' | 'large',
          newOptions.shape as 'circular' | 'square' | 'random' | number,
          adjustmentOptions,
        );
        break;
      // Noise adjustments
      case 'add-noise':
        if (typeof newOptions.amount !== 'number' || typeof newOptions.distribution !== 'string') break;
        abra.noise(layer, newOptions.amount, newOptions.distribution as 'uniform' | 'gaussian', adjustmentOptions);
        break;
      case 'despeckle':
        if (typeof newOptions.radius !== 'number' || typeof newOptions.threshold !== 'number') break;
        abra.despeckle(layer, newOptions.radius, newOptions.threshold, adjustmentOptions);
        break;
      case 'median':
        if (typeof newOptions.radius !== 'number') break;
        abra.median(layer, newOptions.radius, adjustmentOptions);
        break;
    }
  }

  onCompositeChanged(project.id);
}

ipcMain.handle('preview-adjustment', async (_event, { type, options }) => previewAdjustment(type, options));
ipcMain.handle('apply-adjustment', async (_event, {}) => applyAdjustment());
ipcMain.handle('cancel-adjustment', async (_event, {}) => cancelAdjustment());
ipcMain.handle('apply-instant-adjustment', async (_event, { adjustmentType }) => {
  applyInstantAdjustment(adjustmentType);
});
