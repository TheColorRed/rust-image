import { LensBlurOptions } from 'alakazam-bindings';
import { ipcMain } from 'electron';
import {
  clearProjectPreview,
  getActiveProject,
  getProjectPreview,
  onCompositeChanged,
  setProjectPreview,
} from '../events/projects.js';

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
      global.alakazam.autoColor(layer);
      break;
    case 'autoTone':
      global.alakazam.autoTone(layer);
      break;
    case 'invert':
      global.alakazam.invert(layer);
      break;
    case 'grayscale':
      global.alakazam.grayscale(layer);
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

    // Apply adjustments
    switch (type) {
      case 'brightness-contrast':
        if (typeof newOptions.brightness !== 'number' || typeof newOptions.contrast !== 'number') break;
        global.alakazam.brightness(layer, newOptions.brightness);
        global.alakazam.contrast(layer, newOptions.contrast);
        break;
      case 'exposure':
        if (
          typeof newOptions.exposure !== 'number' ||
          typeof newOptions.offset !== 'number' ||
          typeof newOptions.gamma !== 'number'
        )
          break;
        global.alakazam.exposure(layer, newOptions.exposure, newOptions.offset, newOptions.gamma);
        break;
      case 'vibrance':
        if (typeof newOptions.vibrance !== 'number' || typeof newOptions.saturation !== 'number') break;
        global.alakazam.vibrance(layer, newOptions.vibrance, newOptions.saturation);
        break;
      // Blur adjustments
      case 'box-blur':
        if (typeof newOptions.radius !== 'number') break;
        global.alakazam.boxBlur(layer, newOptions.radius);
        break;
      case 'gaussian-blur':
        if (typeof newOptions.radius !== 'number') break;
        global.alakazam.gaussianBlur(layer, newOptions.radius);
        break;
      case 'lens-blur':
        if (typeof newOptions === 'undefined') break;
        global.alakazam.lensBlur(layer, newOptions as unknown as LensBlurOptions);
        break;
      case 'motion-blur':
        if (typeof newOptions.angle !== 'number' || typeof newOptions.distance !== 'number') break;
        global.alakazam.motionBlur(layer, newOptions.angle, newOptions.distance);
        break;
      case 'surface-blur':
        if (typeof newOptions.radius !== 'number' || typeof newOptions.threshold !== 'number') break;
        global.alakazam.surfaceBlur(layer, newOptions.radius, newOptions.threshold);
        break;
      // Distort adjustments
      case 'pinch':
        if (typeof newOptions.amount !== 'number') break;
        // convert number to to -1.0 to 1.0 range
        const pinchAmount = Math.max(-100, Math.min(100, newOptions.amount as number)) / 100;
        global.alakazam.pinch(layer, pinchAmount);
        break;
      case 'ripple':
        if (typeof newOptions.amount !== 'number' || typeof newOptions.size !== 'string') break;
        const rippleAmount = Math.max(-100, Math.min(100, newOptions.amount as number)) / 100;
        global.alakazam.ripple(
          layer,
          rippleAmount,
          newOptions.size as 'small' | 'medium' | 'large',
          newOptions.shape as 'circular' | 'square' | 'random' | number,
        );
        break;
      // Noise adjustments
      case 'add-noise':
        if (typeof newOptions.amount !== 'number' || typeof newOptions.distribution !== 'string') break;
        global.alakazam.noise(layer, newOptions.amount, newOptions.distribution as 'uniform' | 'gaussian');
        break;
      case 'despeckle':
        if (typeof newOptions.radius !== 'number' || typeof newOptions.threshold !== 'number') break;
        global.alakazam.despeckle(layer, newOptions.radius, newOptions.threshold);
        break;
      case 'median':
        if (typeof newOptions.radius !== 'number') break;
        global.alakazam.median(layer, newOptions.radius);
        break;
      default:
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
