import {
  addPluginFilePicker,
  addPluginFontPicker,
  addPluginHeader,
  addPluginNumber,
  addPluginSelect,
  addPluginSize,
  addPluginSlider,
  addPluginText,
  addPluginToggle,
  type PluginFilePickerOptions,
  type PluginFontPickerOptions,
  type PluginPicturesOptions,
  addPluginPictures,
  type PluginHeaderOptions,
  type PluginNumberOptions,
  type PluginSelectOptions,
  type PluginSizeOptions,
  type PluginSliderOptions,
  type PluginTextOptions,
  type PluginToggleOptions,
} from '@/src/lib/plugins/plugin-inspector';
import type { ScriptModules } from '@/src/lib/plugins/plugin-script';
import * as sdk from '@/src/lib/plugins/sdk-runtime';
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
import * as typography from '@abra/core/typography';
import type { Observable, Subscription } from 'rxjs';

/**
 * The modules a plugin script can import: the Abra library, in the same parts the app imports it by, and `@alakazam/sdk` (see
 * `sdk-runtime.ts`). A script that imports anything else does not load.
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
  '@abra/core/typography': typography,
  '@alakazam/sdk': sdk,
};

/** Give each plugin a toolkit instance whose key helper is scoped to its manifest ID, and whose event subscriptions end with `subscriptions`. */
export const pluginModules = (
  pluginId: string,
  subscriptions: Subscription,
  inspector: {
    filePicker: (label: string, options: PluginFilePickerOptions) => void;
    slider: (label: string, options: PluginSliderOptions) => void;
    header: (text: string, options?: PluginHeaderOptions) => void;
    size: (label: string, options?: PluginSizeOptions) => void;
    select: (label: string, options: PluginSelectOptions) => void;
    number: (label: string, options?: PluginNumberOptions) => void;
    toggle: (label: string, options?: PluginToggleOptions) => void;
    text: (label: string, options?: PluginTextOptions) => void;
    fontPicker: (label: string, options?: PluginFontPickerOptions) => void;
    pictures: (label: string, options: PluginPicturesOptions) => void;
  } = {
    filePicker: (label, options) => addPluginFilePicker(pluginId, label, options),
    slider: (label, options) => addPluginSlider(pluginId, label, options),
    header: (text, options) => addPluginHeader(pluginId, text, options),
    size: (label, options) => addPluginSize(pluginId, label, options),
    select: (label, options) => addPluginSelect(pluginId, label, options),
    number: (label, options) => addPluginNumber(pluginId, label, options),
    toggle: (label, options) => addPluginToggle(pluginId, label, options),
    text: (label, options) => addPluginText(pluginId, label, options),
    fontPicker: (label, options) => addPluginFontPicker(pluginId, label, options),
    pictures: (label, options) => addPluginPictures(pluginId, label, options),
  },
): ScriptModules => {
  // Every subscription a plugin makes is added to `subscriptions`, which the caller cancels when the plugin closes.
  const events = Object.fromEntries(
    Object.entries(sdk.app.events).map(([name, stream$]) => [
      name,
      {
        subscribe: (next: (value: never) => void) => {
          const subscription = (stream$ as Observable<never>).subscribe(next);
          subscriptions.add(subscription);
          return subscription;
        },
      },
    ]),
  ) as unknown as typeof sdk.app.events;
  const pluginApi = {
    ...sdk,
    app: { ...sdk.app, events },
    toolKey: (id: string) => `${pluginId}.${id}`,
    // `refresh` and `open` are the same for every plugin; the controls are the plugin's own.
    inspector: { refresh: sdk.inspector.refresh, open: sdk.inspector.open, ...inspector },
  };
  return { ...BASE_PLUGIN_MODULES, '@alakazam/sdk': pluginApi, '@alakazam/plugin-toolkit': pluginApi };
};
