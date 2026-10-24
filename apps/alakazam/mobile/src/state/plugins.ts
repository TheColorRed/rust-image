import { BehaviorSubject } from 'rxjs';

/** Bumped when a plugin adds or takes away controls, so what shows the controls (the editor's panel) reads them again. */
export const pluginControlsVersion = new BehaviorSubject(0);
export const pluginControlsVersion$ = pluginControlsVersion.asObservable();

/**
 * A plugin of the catalog that adds no AI model (a plugin of controls, say), as the Plugins screen lists it. A plugin that adds models is
 * listed with its models (see `AiModelListing`), because a person downloads the models.
 */
export type PluginListing = {
  /** The plugin's id. */
  id: string;
  name: string;
  description: string;
  version: string;
  /** `false` keeps it out of the list until it is installed. */
  enabled: boolean;
  /** Where its `plugin.yml` is and what it hashes to, as the catalog says. */
  manifestUrl: string;
  manifestSha256: string;
  /** What installing it downloads, in bytes, from the server. 0 when it did not say. */
  downloadSizeBytes: number;
  /** Whether this version of it is installed. */
  downloaded: boolean;
};

/** The plugins from the last catalog read that add no model. Empty until the catalog has been read once in this run of the app. */
export const pluginList = new BehaviorSubject<PluginListing[]>([]);
export const pluginList$ = pluginList.asObservable();

/** A plugin that is installed but did not load, and why, in words for a person. */
export type PluginProblem = { plugin: string; message: string };

/** The plugins that could not be loaded the last time they were. Empty when all loaded. */
export const pluginProblems = new BehaviorSubject<PluginProblem[]>([]);
export const pluginProblems$ = pluginProblems.asObservable();
