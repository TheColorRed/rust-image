import { scanPlugins } from '@abra/core';
import {
  aiModelPath,
  deleteAiModel,
  deletePlugin,
  downloadAiModel,
  downloadedAiModelBytes,
  installPlugin,
  pluginsFolder,
} from '@alakazam/mobile';
import { parse } from 'yaml';
import { installPlan, type CatalogPlugin } from '@/src/lib/ai/plugin-dependencies';
import { errorMessage } from '@/src/lib/utils/error-message';
import { aiModelList, type AiModelListing, type AiModelSource } from '@/src/state/ai-models';

/** The catalog of AI models. The app owns it: the native side is only told about one model at a time. */
const MODEL_CATALOG_URL = 'https://alakazam-image-editor.s3.us-east-1.amazonaws.com/plugins/catalog.yml';

/** The download size of each model file, asked of the server once. */
const downloadSizes = new Map<string, number>();

/** The download size of each plugin, in bytes, from the catalog read. A plugin's size on this device is the same. */
const pluginBytes = new Map<string, number>();

/**
 * Whether a model is on this device, from the files. Works offline. A plugin is on this device when that version of it is installed.
 * Before a plugin installed here, a model that is now a plugin was downloaded as a file, so one that is on this device that way counts too:
 * it works as it is, and there is no reason to download it again.
 */
const onDevice = (id: string, source: AiModelSource) => {
  if (source.type === 'file') {
    const sizeBytes = Number(downloadedAiModelBytes(source.file));
    return { downloaded: sizeBytes > 0, sizeBytes };
  }
  const installed = scanPlugins(pluginsFolder()).plugins.find(plugin => plugin.id === source.pluginId);
  const sizeBytes = pluginBytes.get(source.manifestUrl) ?? 0;
  if (installed) return { downloaded: installed.version === source.version, sizeBytes };
  try {
    aiModelPath(id);
    return { downloaded: true, sizeBytes };
  } catch {
    return { downloaded: false, sizeBytes: 0 };
  }
};

/** Brings what is on this device up to date in the list that is kept (after a download or a delete). Works offline. */
export const syncAiModels = () =>
  aiModelList.next(aiModelList.value.map(model => ({ ...model, ...onDevice(model.id, model.source) })));

/** The plugins the catalog lists, by id. It is the lookup that finds a plugin's dependencies. Filled by every read of the catalog. */
const catalogPlugins = new Map<string, CatalogPlugin>();

/** The ids of the plugins that a plugin of the catalog depends on, from its `plugin.yml`. */
const dependenciesOf = async (plugin: CatalogPlugin): Promise<string[]> => {
  const response = await fetch(plugin.uri);
  if (!response.ok) throw new Error(`The plugin ${plugin.id} could not be downloaded (${response.status}).`);
  const dependencies: unknown = parse(await response.text())?.dependencies;
  return Array.isArray(dependencies) ? dependencies.map((dependency: { id?: unknown }) => String(dependency?.id)) : [];
};

/**
 * Installs a plugin of the catalog, and first the plugins it depends on that are not on this device, and what those depend on. The catalog
 * says where each is. `listener` hears how far it has got over all of them, from 0 to 1. Fails with a message for a person when a plugin
 * that is needed is not in the catalog.
 */
export const installPluginWithDependencies = async (id: string, listener: { update: (progress: number) => void }): Promise<void> => {
  const installed = new Map(
    scanPlugins(pluginsFolder()).plugins.map(plugin => [
      plugin.id,
      { usable: plugin.missingDependencies.length === 0, missingDependencies: plugin.missingDependencies },
    ]),
  );
  const plan = await installPlan(id, catalogPlugins, installed, dependenciesOf);
  for (const [index, plugin] of plan.entries()) {
    await installPlugin(plugin.uri, plugin.sha256, { update: share => listener.update((index + share) / plan.length) });
  }
  listener.update(1);
};

/** Downloads a model, from whatever it comes from, telling `listener` how far it has got from 0 to 1. */
export const downloadModel = (model: AiModelListing, listener: { update: (progress: number) => void }): Promise<void> =>
  model.source.type === 'file'
    ? downloadAiModel(model.source.file, listener)
    : installPluginWithDependencies(model.source.pluginId, listener);

/** Deletes what a model downloaded. For a plugin that is the plugin, and so every model it adds, and a copy downloaded as a file before. */
export const deleteModel = (model: AiModelListing): void => {
  if (model.source.type === 'plugin') deletePlugin(model.source.pluginId);
  deleteAiModel(model.id);
};

/** Reads the catalog, keeps the list of its models and returns it. Fails when the catalog cannot be read. */
export const refreshAiModels = async (): Promise<AiModelListing[]> => {
  const response = await fetch(MODEL_CATALOG_URL);
  if (!response.ok) throw new Error(`The model catalog could not be downloaded (${response.status}).`);
  const parsed = parse(await response.text());
  // `models` lists the models as files, for the apps that came before plugins. A catalog of plugins only does not need it.
  const items: unknown[] = Array.isArray(parsed?.models) ? parsed.models : [];
  if (!Array.isArray(parsed?.plugins) && items.length === 0) throw new Error('The catalog lists no plugins.');
  // The plugins by id, for finding the plugins a plugin depends on. A plugin without an id cannot be found, so it is left out.
  catalogPlugins.clear();
  for (const item of Array.isArray(parsed?.plugins) ? parsed.plugins : []) {
    if (typeof item?.id === 'string' && typeof item.uri === 'string' && /^[0-9a-fA-F]{64}$/.test(String(item.sha256))) {
      catalogPlugins.set(item.id, { id: item.id, uri: item.uri, sha256: String(item.sha256).toLowerCase() });
    } else if (__DEV__) {
      console.warn('[plugins] a plugin in the catalog needs its `id`, `uri` and SHA-256 `sha256`:', JSON.stringify(item));
    }
  }

  const models = await Promise.all(
    items.map(async (entry): Promise<AiModelListing> => {
      const item = entry as Record<string, unknown>;
      const text = (key: string) => {
        if (typeof item[key] !== 'string') throw new Error(`A model in the catalog has no \`${key}\`.`);
        return item[key];
      };
      const hash = (url: string, value: string) => {
        if (!/^[0-9a-fA-F]{64}$/.test(value)) throw new Error(`The catalog hash for ${url} is not a SHA-256.`);
        return value.toLowerCase();
      };
      const url = text('uri');
      const manifestUrl = typeof item.manifest_uri === 'string' ? item.manifest_uri : undefined;
      const manifestHash = typeof item.manifest_sha256 === 'string' ? item.manifest_sha256 : undefined;
      if ((manifestUrl === undefined) !== (manifestHash === undefined)) {
        throw new Error(`The model ${url} needs both \`manifest_uri\` and \`manifest_sha256\`, or neither.`);
      }
      if (item.enabled !== undefined && typeof item.enabled !== 'boolean') {
        throw new Error(`The \`enabled\` of the model ${url} must be true or false.`);
      }
      // `categories` has a `subject` list (what the model finds) and a `kind` list (what it gives back and how it works), both optional.
      const categories = (item.categories ?? {}) as Record<string, unknown>;
      const category = (key: 'subject' | 'kind'): string[] => {
        const values = categories[key] ?? [];
        if (!Array.isArray(values) || values.some(value => typeof value !== 'string')) {
          throw new Error(`The \`${key}\` of the model ${url} must be a list of text.`);
        }
        return values;
      };
      const file = {
        role: text('role'),
        url,
        sha256: hash(url, text('sha256')),
        manifestUrl,
        manifestSha256: manifestUrl && manifestHash ? hash(manifestUrl, manifestHash) : undefined,
      };
      const source: AiModelSource = { type: 'file', file };
      const downloadSizeBytes = await sizeOf(url);
      return {
        id: file.role,
        title: text('title'),
        description: typeof item.description === 'string' ? item.description : '',
        enabled: item.enabled !== false,
        subjects: category('subject'),
        kinds: category('kind'),
        source,
        downloadSizeBytes,
        ...onDevice(file.role, source),
      };
    }),
  );
  // The plugins the catalog lists, each a `plugin.yml` that says which models it adds. A plugin that cannot be read is left out, so one
  // bad entry never takes the rest of the catalog away.
  const plugins = await Promise.all(
    (Array.isArray(parsed?.plugins) ? parsed.plugins : []).map((item: Record<string, unknown>) =>
      pluginModels(item).catch(error => {
        if (__DEV__) console.warn('[plugins] a plugin in the catalog could not be read:', errorMessage(error));
        return [];
      }),
    ),
  );
  // A model a plugin adds takes the place of the same model listed as a file, which is only still listed for the apps that came before
  // plugins and cannot read the `plugins` list.
  const added = plugins.flat();
  const all = [...models.filter(model => !added.some(other => other.id === model.id)), ...added];
  aiModelList.next(all);
  return all;
};

/** The download size of a file, asked of the server once. 0 when the server does not say. */
const sizeOf = async (url: string): Promise<number> => {
  if (!downloadSizes.has(url)) {
    const size = await fetch(url, { method: 'HEAD' })
      .then(head => Number(head.headers.get('content-length')) || 0)
      .catch(() => 0);
    downloadSizes.set(url, size);
  }
  return downloadSizes.get(url) ?? 0;
};

/**
 * The models a plugin of the catalog adds, from its `plugin.yml`. The catalog gives the file's URL and hash; the hash is checked when the
 * plugin is installed, by the native side, together with the hash of every file the plugin lists.
 */
const pluginModels = async (item: Record<string, unknown>): Promise<AiModelListing[]> => {
  const manifestUrl = item.uri;
  const manifestSha256 = item.sha256;
  if (typeof manifestUrl !== 'string' || typeof manifestSha256 !== 'string' || !/^[0-9a-fA-F]{64}$/.test(manifestSha256)) {
    throw new Error('A plugin in the catalog needs its `uri` and a SHA-256 `sha256`.');
  }
  const response = await fetch(manifestUrl);
  if (!response.ok) throw new Error(`The plugin ${manifestUrl} could not be downloaded (${response.status}).`);
  const manifest = parse(await response.text());
  const strings = (value: unknown): string[] => (Array.isArray(value) ? value.filter((v): v is string => typeof v === 'string') : []);
  if (typeof manifest?.id !== 'string' || (typeof manifest.version !== 'string' && typeof manifest.version !== 'number')) {
    throw new Error(`The plugin ${manifestUrl} has no id or version.`);
  }
  // The catalog is looked up by id, so the id it gives has to be the plugin's own.
  if (item.id !== manifest.id) throw new Error(`The catalog lists ${manifestUrl} as ${String(item.id)}, but it is ${manifest.id}.`);
  const base = manifestUrl.slice(0, manifestUrl.lastIndexOf('/'));
  const files: { path: string }[] = Array.isArray(manifest.files) ? manifest.files : [];
  const sizes = await Promise.all(files.map(file => sizeOf(`${base}/${file.path}`)));
  const bytes = sizes.reduce((total, size) => total + size, 0);
  pluginBytes.set(manifestUrl, bytes);
  const source: AiModelSource = {
    type: 'plugin',
    pluginId: manifest.id,
    version: String(manifest.version),
    manifestUrl,
    manifestSha256: manifestSha256.toLowerCase(),
  };
  const models: Record<string, unknown>[] = Array.isArray(manifest.contributes?.models) ? manifest.contributes.models : [];
  return models.map(model => ({
    id: String(model.id),
    title: String(model.title ?? model.id),
    description: typeof model.description === 'string' ? model.description : '',
    enabled: item.enabled !== false,
    subjects: strings(model.subjects),
    kinds: strings(model.capabilities),
    source,
    downloadSizeBytes: bytes,
    ...onDevice(String(model.id), source),
  }));
};
