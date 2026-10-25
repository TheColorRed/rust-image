import * as abra from '@abra/core';
import * as adjustments from '@abra/core/adjustments';
import * as beauty from '@abra/core/beauty';
import * as blend from '@abra/core/blend';
import * as canvas from '@abra/core/canvas';
import * as color from '@abra/core/color';
import * as drawing from '@abra/core/drawing';
import * as fill from '@abra/core/fill';
import * as filters from '@abra/core/filters';
import * as geometry from '@abra/core/geometry';
import * as gradient from '@abra/core/gradient';
import * as image from '@abra/core/image';
import * as mask from '@abra/core/mask';
import * as tools from '@abra/core/tools';
import * as transform from '@abra/core/transform';
import * as hostApi from '@/src/lib/plugins/host-api';
import type { ScriptModules } from '@/src/lib/plugins/plugin-script';

/**
 * The modules a plugin script can import: the Abra library, in the same parts the app imports it by, and `@alakazam/plugin-toolkit` (see
 * `host-api.ts`). A script that imports anything else does not load.
 */
const BASE_PLUGIN_MODULES: ScriptModules = {
  '@abra/core': abra,
  '@abra/core/adjustments': adjustments,
  '@abra/core/beauty': beauty,
  '@abra/core/blend': blend,
  '@abra/core/canvas': canvas,
  '@abra/core/color': color,
  '@abra/core/drawing': drawing,
  '@abra/core/fill': fill,
  '@abra/core/filters': filters,
  '@abra/core/geometry': geometry,
  '@abra/core/gradient': gradient,
  '@abra/core/image': image,
  '@abra/core/mask': mask,
  '@abra/core/tools': tools,
  '@abra/core/transform': transform,
  '@alakazam/plugin-toolkit': hostApi,
};

/** Give each plugin a toolkit instance whose key helper is scoped to its manifest ID. */
export const pluginModules = (pluginId: string): ScriptModules => ({
  ...BASE_PLUGIN_MODULES,
  '@alakazam/plugin-toolkit': {
    ...hostApi,
    toolKey: (id: string) => `${pluginId}.${id}`,
  },
});
