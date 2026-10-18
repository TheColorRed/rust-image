import { downloadedAiModelBytes } from '@alakazam/mobile';
import { parse } from 'yaml';
import { aiModelList, type AiModelListing } from '@/src/state/ai-models';

/** The catalog of AI models. The app owns it: the native side is only told about one model at a time. */
const MODEL_CATALOG_URL = 'https://alakazam-image-editor.s3.us-east-1.amazonaws.com/catalog.yml';

/** The download size of each model file, asked of the server once. */
const downloadSizes = new Map<string, number>();

/** Whether a model is on this device, from the files. Works offline. */
const onDevice = (file: AiModelListing['file']) => {
  const sizeBytes = Number(downloadedAiModelBytes(file));
  return { downloaded: sizeBytes > 0, sizeBytes };
};

/** Brings what is on this device up to date in the list that is kept (after a download or a delete). Works offline. */
export const syncAiModels = () =>
  aiModelList.next(aiModelList.value.map(model => ({ ...model, ...onDevice(model.file) })));

/** Reads the catalog, keeps the list of its models and returns it. Fails when the catalog cannot be read. */
export const refreshAiModels = async (): Promise<AiModelListing[]> => {
  const response = await fetch(MODEL_CATALOG_URL);
  if (!response.ok) throw new Error(`The model catalog could not be downloaded (${response.status}).`);
  const items: unknown = parse(await response.text())?.models;
  if (!Array.isArray(items)) throw new Error('The model catalog has no `models` list.');

  const models = await Promise.all(
    items.map(async (item: Record<string, unknown>): Promise<AiModelListing> => {
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
      if (!downloadSizes.has(url)) {
        const size = await fetch(url, { method: 'HEAD' })
          .then(head => Number(head.headers.get('content-length')) || 0)
          .catch(() => 0);
        downloadSizes.set(url, size);
      }
      return {
        id: file.role,
        title: text('title'),
        description: typeof item.description === 'string' ? item.description : '',
        enabled: item.enabled !== false,
        subjects: category('subject'),
        kinds: category('kind'),
        file,
        downloadSizeBytes: downloadSizes.get(url) ?? 0,
        ...onDevice(file),
      };
    }),
  );
  aiModelList.next(models);
  return models;
};
