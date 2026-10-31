/**
 * Runtime values the app offers a plugin script as the module `@alakazam/sdk`. The public types live in the workspace npm package
 * so plugin authors can install them without the app source.
 *
 * Keep it small and stable. Anything added here is something every plugin can depend on.
 */
import { map } from 'rxjs/operators';
import { Observable } from 'rxjs';
import { computePreviewImagePoint } from '@/src/lib/editor/preview-coordinates';
import { previewBox, previewPressed$, previewRotationValue, previewScaleValue, previewTranslateValue } from '@/src/state/gestures';
import { pluginTouch$, pluginTouchListeners, type PluginTouch } from '@/src/state/plugin-touch';
import { missingModels } from '@/src/lib/ai/ai-model-availability';
import { framedPeople } from '@/src/lib/ai/analysis';
import { aiModelLifetimes, registerAiModel, releaseAiModel, runWithAiModel } from '@/src/lib/ai/model-warmth';
import { commitRetouch } from '@/src/lib/editor/editor-history';
import { flushFullReplay } from '@/src/lib/editor/editor-replay';
import { saveOpenProject } from '@/src/lib/editor/editor-session';
import { release } from '@/src/lib/native/native-handles';
import {
  addPluginFilePicker,
  addPluginHeader,
  addPluginNumber,
  addPluginSelect,
  addPluginSize,
  addPluginSlider,
  type PluginFilePickerOptions,
  type PluginHeaderOptions,
  type PluginNumberOptions,
  type PluginSelectOptions,
  type PluginSizeOptions,
  type PluginSliderOptions,
} from '@/src/lib/plugins/plugin-inspector';
import { aiModelList, aiModelsRequested } from '@/src/state/ai-models';
import { aiProgress, aiProgress$ } from '@/src/state/ai-progress';
import { people$ } from '@/src/state/analysis';
import { adjustments$ } from '@/src/state/edits';
import { timeline$ } from '@/src/state/history';
import { areaRings, areas, areas$, setArea } from '@/src/state/draw-area';
import { historyReady, historyReady$ } from '@/src/state/project-session';
import { busy, currentImage } from '@/src/state/session';
import { setToolAiModel, toolAiModelSelections } from '@/src/state/tool-ai-models';
import { setToolMessage } from '@/src/state/tool-messages';
import { AiModel, type AiModel as AiModelType } from '@abra/core';
import { type ImageLike } from '@abra/core/image';
import { type MaskLike } from '@abra/core/mask';
import { aiModelPath, runAiModelAsync, runAiModelGuidedAsync, type AbraImageLike } from '@alakazam/mobile';
import Rand from 'rand-seed';
export { belongsTo, findFaces } from '@/src/lib/ai/faces';
export { paintSheet, type Painting } from '@/src/lib/beauty/layers';
export { makeupMasks as faceMasks } from '@/src/lib/beauty/makeup-masks';
export { derivedFor } from '@/src/lib/editor/editor-preloads';
export { areaMask } from '@/src/lib/native/area-mask';
export { owning, release } from '@/src/lib/native/native-handles';
export { swatchesOf } from '@/src/lib/project/edit-sections/swatches';
export { setAiPendingNotice } from '@/src/state/ai-progress';
export type { ControlScript, EditControl, SliderControl, ToolContext } from '@alakazam/sdk';
/**
 * Creates a seeded random number generator using the provided seed string.
 * @param s The seed string used to initialize the random number generator.
 * @returns A seeded random number generator instance.
 */
export const seededRandom = (s: string) => new Rand(s);

/** Plugin-authored controls that the host renders in the plugin editor inspector. */
export const inspector = {
  filePicker: (label: string, options: PluginFilePickerOptions) => addPluginFilePicker('unscoped', label, options),
  slider: (label: string, options: PluginSliderOptions) => addPluginSlider('unscoped', label, options),
  header: (text: string, options?: PluginHeaderOptions) => addPluginHeader('unscoped', text, options),
  size: (label: string, options?: PluginSizeOptions) => addPluginSize('unscoped', label, options),
  select: (label: string, options: PluginSelectOptions) => addPluginSelect('unscoped', label, options),
  number: (label: string, options?: PluginNumberOptions) => addPluginNumber('unscoped', label, options),
};

/** Generic app capabilities for plugin controls: editor state, AI models and runs, history, and tool feedback. */
/** Where a point in the preview's pixels is on the open photo, in the photo's pixels, or null for both when it is off the photo. */
const onPhoto = (point: { x: number; y: number }) => {
  const image = currentImage.value as AbraImageLike | null;
  const box = previewBox.value;
  const at =
    image && box
      ? computePreviewImagePoint(image.width(), image.height(), box, previewScaleValue.value, previewTranslateValue.value, point, previewRotationValue.value)
      : null;
  return { imageX: at?.x ?? null, imageY: at?.y ?? null };
};

export const app = {
  editor: {
    currentImage: () => currentImage.value,
    hasHistory: () => historyReady.value,
    flush: () => flushFullReplay(),
    area: (key: string) => areaRings(areas.value, key),
    setArea: (key: string, rings: number[][][]) => setArea(key, rings),
    setBusy: (value: boolean) => busy.next(value),
    replaceImage: (image: AbraImageLike, result: ImageLike) =>
      image.replaceWith(result as Parameters<AbraImageLike['replaceWith']>[0]),
    subjects: () => framedPeople() ?? [],
  },
  /** Read-only streams of what changes in the editor. `plugin-host.ts` hands each plugin a copy that cancels its subscriptions when it closes. */
  events: {
    areas: areas$,
    history: timeline$,
    historyReady: historyReady$,
    adjustments: adjustments$,
    people: people$,
    aiProgress: aiProgress$,
    taps: previewPressed$.pipe(map(tap => ({ x: tap.x, y: tap.y, ...onPhoto(tap) }))),
    touches: new Observable<PluginTouch>(subscriber => {
      pluginTouchListeners.next(pluginTouchListeners.value + 1);
      const subscription = pluginTouch$.subscribe(subscriber);
      return () => {
        subscription.unsubscribe();
        pluginTouchListeners.next(pluginTouchListeners.value - 1);
      };
    }).pipe(map(touch => ({ type: touch.type, x: touch.x, y: touch.y, ...onPhoto(touch) }))),
  },
  ai: {
    models: () => aiModelList.value,
    selectedModel: (key: string) => toolAiModelSelections.value[key] ?? null,
    setSelectedModel: (key: string, role: string) => setToolAiModel(key, role),
    missing: (roles: readonly string[]) => missingModels(roles),
    requestModels: (title: string, roles: readonly string[], onReady: () => void) =>
      aiModelsRequested.next({ title, roles, onReady }),
    prepareModel: (owner: string, role: string) =>
      registerAiModel({ owner, role, lifetime: aiModelLifetimes.multiShot }),
    releaseModel: (owner: string, role?: string) => releaseAiModel(owner, role),
    withModel: <T>(owner: string, role: string, work: (model: AiModel) => Promise<T>) =>
      runWithAiModel({ owner, role, lifetime: aiModelLifetimes.multiShot }, model => work(model as AiModel)),
    /** Runs a model file with a plugin-supplied manifest, freeing the native model when the work ends. */
    withManifest: async <T>(role: string, manifest: string, work: (model: AiModelType) => Promise<T>): Promise<T> => {
      const model = AiModel.loadWithManifest(aiModelPath(role), manifest);
      try {
        return await work(model as AiModelType);
      } finally {
        release(model);
      }
    },
    /** Runs an image model directly and returns its typed model result. */
    run: (model: AiModelType, image: ImageLike) =>
      runAiModelAsync(model, image as Parameters<typeof runAiModelAsync>[1]),
    runGuided: (model: AiModel, image: ImageLike, mask: MaskLike) =>
      runAiModelGuidedAsync(
        model,
        image as Parameters<typeof runAiModelGuidedAsync>[1],
        mask as Parameters<typeof runAiModelGuidedAsync>[2],
      ),
    withProgress: async <T>(key: string, work: () => Promise<T>): Promise<T> => {
      aiProgress.next({ ...aiProgress.value, [key]: null });
      let failed = false;
      try {
        return await work();
      } catch (error) {
        failed = true;
        throw error;
      } finally {
        const { [key]: _finished, ...others } = aiProgress.value;
        aiProgress.next(failed ? others : { ...aiProgress.value, [key]: 1 });
      }
    },
  },
  history: {
    commitRetouch: (change: { type: string; [key: string]: unknown }) =>
      commitRetouch(change as Parameters<typeof commitRetouch>[0]),
    save: () => saveOpenProject(),
  },
  tools: { setMessage: (key: string, message: string | null) => setToolMessage(key, message) },
};

/**
 * What a control script of a plugin exports as its default: a function that returns the control. The plugin's `plugin.yml` says where
 * the control goes and what it is called, so the control does not.
 */
