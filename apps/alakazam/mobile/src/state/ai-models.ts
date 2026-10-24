import { BehaviorSubject } from 'rxjs';

/** One AI model in the catalog (`catalog.yml`, read by `lib/ai-model-catalog`), and whether it is on this device. */
export type AiModelListing = {
  /** What the model is used for; names it on this device. */
  id: string;
  /** Name shown to people. */
  title: string;
  /** What the model does. */
  description: string;
  /** `false` keeps the model out of the list until it is downloaded. */
  enabled: boolean;
  /** What the model finds (`people`, `face`, `edges`...), from the catalog's `categories.subject`. */
  subjects: string[];
  /** What the model gives back and how it works (`mask`, `instances`, `labels`, `landmarks`, `guided`...), from `categories.kind`. */
  kinds: string[];
  /** Where the model comes from, which is what the app tells the native side to download, and to look for on this device. */
  source: AiModelSource;
  /** Size of the file to download in bytes, from the server. 0 when it did not say. */
  downloadSizeBytes: number;
  /** Whether the model, and its manifest, are on this device. */
  downloaded: boolean;
  /** Size of the downloaded file in bytes, or 0 when it is not downloaded. */
  sizeBytes: number;
};

/**
 * Where a model comes from. A `file` is one model file with its manifest. A `plugin` is a plugin that adds the model: installing it
 * downloads every file the plugin lists (see `abra::plugin`), and deleting it removes them all, including the other models it adds.
 */
export type AiModelSource =
  | {
      type: 'file';
      file: { role: string; url: string; sha256: string; manifestUrl: string | undefined; manifestSha256: string | undefined };
    }
  | { type: 'plugin'; pluginId: string; version: string; manifestUrl: string; manifestSha256: string };

/**
 * The AI models from the last catalog read, kept here so the plugins screen shows them as soon as it opens and then
 * refreshes them. Empty until the catalog has been read once in this run of the app.
 */
export const aiModelList = new BehaviorSubject<AiModelListing[]>([]);
export const aiModelList$ = aiModelList.asObservable();

/** Bumped after AI models are downloaded, so anything waiting on them can try again. */
export const aiModelsVersion = new BehaviorSubject(0);
export const aiModelsVersion$ = aiModelsVersion.asObservable();

/** Set when the download offer was declined, so it is not shown again until the app restarts. */
export const aiModelsOfferDeclined = new BehaviorSubject(false);

/**
 * An action that needs AI models that are not on this device was tapped. The prompt asks to download them; `onReady` runs
 * the action again once they are. Cancelling drops the request and nothing is downloaded.
 */
export type AiModelsRequest = { title: string; roles: readonly string[]; onReady: () => void };
export const aiModelsRequested = new BehaviorSubject<AiModelsRequest | null>(null);
export const aiModelsRequested$ = aiModelsRequested.asObservable();
