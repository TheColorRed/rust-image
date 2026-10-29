import * as hostApi from '@/src/lib/plugins/host-api';
import {
  addPluginFilePicker,
  addPluginHeader,
  addPluginSlider,
  addPluginNumber,
  addPluginSelect,
  addPluginSize,
  addPluginToggle,
  type PluginFilePickerOptions,
  type PluginHeaderOptions,
  type PluginSliderOptions,
  type PluginNumberOptions,
  type PluginSelectOptions,
  type PluginSizeOptions,
  type PluginToggleOptions,
} from '@/src/lib/plugins/plugin-inspector';
import type { ScriptModules } from '@/src/lib/plugins/plugin-script';
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

/**
 * The modules a plugin script can import: the Abra library, in the same parts the app imports it by, and `@alakazam/plugin` (see
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
  '@alakazam/plugin': hostApi,
  // Keep already-published plugins working while new plugin projects use @alakazam/plugin.
  '@alakazam/plugin-toolkit': hostApi,
};

/** Give each plugin a toolkit instance whose key helper is scoped to its manifest ID. */
export const pluginModules = (
  pluginId: string,
  inspector: {
    filePicker: (label: string, options: PluginFilePickerOptions) => void;
    slider: (label: string, options: PluginSliderOptions) => void;
    header: (text: string, options?: PluginHeaderOptions) => void;
    size: (label: string, options?: PluginSizeOptions) => void;
    select: (label: string, options: PluginSelectOptions) => void;
    number: (label: string, options?: PluginNumberOptions) => void;
    toggle: (label: string, options?: PluginToggleOptions) => void;
  } = {
    filePicker: (label, options) => addPluginFilePicker(pluginId, label, options),
    slider: (label, options) => addPluginSlider(pluginId, label, options),
    header: (text, options) => addPluginHeader(pluginId, text, options),
    size: (label, options) => addPluginSize(pluginId, label, options),
    select: (label, options) => addPluginSelect(pluginId, label, options),
    number: (label, options) => addPluginNumber(pluginId, label, options),
    toggle: (label, options) => addPluginToggle(pluginId, label, options),
  },
): ScriptModules => {
  const pluginApi = {
    ...hostApi,
    toolKey: (id: string) => `${pluginId}.${id}`,
    inspector,
  };
  return { ...BASE_PLUGIN_MODULES, '@alakazam/plugin': pluginApi, '@alakazam/plugin-toolkit': pluginApi };
};
