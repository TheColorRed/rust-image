/**
 * Runtime values the app offers a plugin script as the module `@alakazam/plugin-toolkit`. The public types live in the workspace npm package
 * so plugin authors can install them without the app source.
 *
 * Keep it small and stable. Anything added here is something every plugin can depend on.
 */
import { type AiModel } from '@abra/core';
import { type ImageLike } from '@abra/core/image';
import { type MaskLike } from '@abra/core/mask';
import { runAiModelGuidedAsync, type AbraImageLike } from '@alakazam/mobile';
import { aiModelLifetimes, registerAiModel, releaseAiModel, runWithAiModel } from '@/src/lib/ai/model-warmth';
import { missingModels } from '@/src/lib/ai/ai-model-availability';
import { flushFullReplay } from '@/src/lib/editor/editor-replay';
import { commitRetouch } from '@/src/lib/editor/editor-history';
import { saveOpenProject } from '@/src/lib/editor/editor-session';
import { aiModelList, aiModelsRequested } from '@/src/state/ai-models';
import { aiProgress } from '@/src/state/ai-progress';
import { areas, areaRings, setArea } from '@/src/state/draw-area';
import { historyReady } from '@/src/state/project-session';
import { busy, currentImage } from '@/src/state/session';
import { setToolMessage } from '@/src/state/tool-messages';
import { setToolAiModel, toolAiModelSelections } from '@/src/state/tool-ai-models';
export { derivedFor } from '@/src/lib/editor/editor-preloads';
export { belongsTo } from '@/src/lib/ai/faces';
export { owning, release } from '@/src/lib/native/native-handles';
export { paintSheet, type Painting } from '@/src/lib/beauty/layers';
export { makeupMasks as faceMasks } from '@/src/lib/beauty/makeup-masks';
export { swatchesOf } from '@/src/lib/project/edit-sections/swatches';
export { setAiPendingNotice } from '@/src/state/ai-progress';
export { areaMask } from '@/src/lib/native/area-mask';

/** Generic app capabilities for plugin controls: editor state, AI models and runs, history, and tool feedback. */
export const app = {
  editor: {
    currentImage: () => currentImage.value,
    hasHistory: () => historyReady.value,
    flush: () => flushFullReplay(),
    area: (key: string) => areaRings(areas.value, key),
    setArea: (key: string, rings: number[][][]) => setArea(key, rings),
    setBusy: (value: boolean) => busy.next(value),
    replaceImage: (image: AbraImageLike, result: ImageLike) => image.replaceWith(result as Parameters<AbraImageLike['replaceWith']>[0]),
  },
  ai: {
    models: () => aiModelList.value,
    selectedModel: (key: string) => toolAiModelSelections.value[key] ?? null,
    setSelectedModel: (key: string, role: string) => setToolAiModel(key, role),
    missing: (roles: readonly string[]) => missingModels(roles),
    requestModels: (title: string, roles: readonly string[], onReady: () => void) =>
      aiModelsRequested.next({ title, roles, onReady }),
    prepareModel: (owner: string, role: string) => registerAiModel({ owner, role, lifetime: aiModelLifetimes.multiShot }),
    releaseModel: (owner: string, role?: string) => releaseAiModel(owner, role),
    withModel: <T>(owner: string, role: string, work: (model: AiModel) => Promise<T>) =>
      runWithAiModel({ owner, role, lifetime: aiModelLifetimes.multiShot }, work),
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
    commitRetouch: (change: { type: string; [key: string]: unknown }) => commitRetouch(change as Parameters<typeof commitRetouch>[0]),
    save: () => saveOpenProject(),
  },
  tools: { setMessage: (key: string, message: string | null) => setToolMessage(key, message) },
};
export type { EditControl, SliderControl, ToolContext, ControlScript } from '@alakazam/plugin-toolkit';

/**
 * What a control script of a plugin exports as its default: a function that returns the control. The plugin's `plugin.yml` says where
 * the control goes and what it is called, so the control does not.
 */
