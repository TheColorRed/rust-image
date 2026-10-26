import { warn } from '@/src/lib/log';
import AsyncStorage from '@react-native-async-storage/async-storage';
import { BehaviorSubject } from 'rxjs';

/** Each tool path and each category or section subtree uses this many recent selections. */
export const TOOLBAR_USAGE_WINDOW = 50;

type UsageEntry = { plugin?: string; lastUsed: string[] };
type ToolbarUsage = Record<string, UsageEntry>;
type KeyedItem = { key: string; label?: string; pluginId?: string; controls?: unknown };

const STORAGE_KEY = 'toolbar-usage';
export const toolbarUsage = new BehaviorSubject<ToolbarUsage>({});
export const toolbarUsage$ = toolbarUsage.asObservable();

let loaded = false;
let writes = Promise.resolve();
let clearRequested = false;
let pruneRequested = false;
let validPathsAfterPrune: ReadonlyMap<string, string | undefined> | undefined;

const save = (usage: ToolbarUsage) => {
  writes = writes
    .then(() => AsyncStorage.setItem(STORAGE_KEY, JSON.stringify(usage)))
    .catch(error => warn('toolbar', 'could not save recent tool use:', error));
};

const validUsage = (value: unknown): ToolbarUsage => {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) return {};
  return Object.fromEntries(
    Object.entries(value).flatMap(([path, entry]) => {
      if (entry === null || typeof entry !== 'object' || !Array.isArray((entry as UsageEntry).lastUsed)) return [];
      const validEntry = entry as UsageEntry;
      const lastUsed = validEntry.lastUsed
        .filter((time): time is string => typeof time === 'string' && !Number.isNaN(Date.parse(time)))
        .sort()
        .slice(-TOOLBAR_USAGE_WINDOW);
      if (!lastUsed.length) return [];
      return [[path, { ...(typeof validEntry.plugin === 'string' ? { plugin: validEntry.plugin } : {}), lastUsed }]];
    }),
  );
};

const pathSlug = (label: string): string =>
  label
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '');

const sectionOf = (path: string): string => path.split('/')[0];

const applyPathPrune = (usage: ToolbarUsage, validPaths: ReadonlyMap<string, string | undefined>): ToolbarUsage =>
  Object.fromEntries(
    Object.entries(usage).filter(([path, entry]) => {
      if (!validPaths.has(path)) return false;
      const currentPlugin = validPaths.get(path);
      return entry.plugin === undefined || entry.plugin === currentPlugin;
    }),
  );

// This is a separate AsyncStorage entry so the history can evolve independently of Settings.
void (async () => {
  let saved: ToolbarUsage = {};
  let needsRewrite = false;
  try {
    const stored = JSON.parse((await AsyncStorage.getItem(STORAGE_KEY)) ?? '{}');
    saved = validUsage(stored);
    needsRewrite = JSON.stringify(saved) !== JSON.stringify(stored);
  } catch (error) {
    warn('toolbar', 'could not read recent tool use:', error);
  }

  if (clearRequested) saved = {};
  if (pruneRequested && validPathsAfterPrune) saved = applyPathPrune(saved, validPathsAfterPrune);
  const combined = { ...saved };
  for (const [path, entry] of Object.entries(toolbarUsage.value)) {
    combined[path] = {
      ...(saved[path]?.plugin || entry.plugin ? { plugin: entry.plugin ?? saved[path]?.plugin } : {}),
      lastUsed: [...(saved[path]?.lastUsed ?? []), ...entry.lastUsed],
    };
  }
  const recent = validUsage(combined);
  toolbarUsage.next(recent);
  loaded = true;
  if (clearRequested || pruneRequested || needsRewrite || Object.keys(recent).length > 0) save(recent);
})();

/** Records a direct selection by its readable path, with plugin ownership when it comes from a plugin. */
export const recordToolbarUse = (path: string, plugin?: string): void => {
  const section = sectionOf(path);
  const current = toolbarUsage.value;
  const latestTime = Object.entries(current)
    .filter(([itemPath]) => sectionOf(itemPath) === section)
    .flatMap(([, entry]) => entry.lastUsed)
    .reduce((latest, time) => Math.max(latest, Date.parse(time)), 0);
  // Keep timestamps strictly increasing within a section, even if two taps share a clock tick.
  const time = new Date(Math.max(Date.now(), latestTime + 1)).toISOString();
  const previous = current[path];
  const next = {
    ...current,
    [path]: {
      ...(plugin ? { plugin } : {}),
      lastUsed: [...(previous?.lastUsed ?? []), time].slice(-TOOLBAR_USAGE_WINDOW),
    },
  };
  toolbarUsage.next(next);
  if (loaded) save(next);
};

/** Clears recent use, returning every toolbar level to its declared order. */
export const clearToolbarUsage = (): void => {
  clearRequested = true;
  toolbarUsage.next({});
  if (loaded) save({});
};

const descendants = (item: KeyedItem): KeyedItem[] => {
  const controls = Array.isArray(item.controls) ? (item.controls as KeyedItem[]) : [];
  return [item, ...controls.flatMap(child => descendants(child))];
};

const uniquePlugin = (items: readonly KeyedItem[]): string | undefined => {
  const plugins = new Set(items.flatMap(item => descendants(item).flatMap(child => (child.pluginId ? [child.pluginId] : []))));
  return plugins.size === 1 ? [...plugins][0] : undefined;
};

const itemPath = (parent: string, item: KeyedItem): string => `${parent}/${pathSlug(item.label ?? item.key)}`;

/** The readable paths present in the editor, with their plugin owner when one is unambiguous. */
export const toolbarPathsFor = (sections: readonly KeyedItem[]): ReadonlyMap<string, string | undefined> => {
  const paths = new Map<string, string | undefined>();
  const visit = (item: KeyedItem, parent = '') => {
    const path = parent ? itemPath(parent, item) : pathSlug(item.label ?? item.key);
    paths.set(path, item.pluginId ?? uniquePlugin([item]));
    for (const child of Array.isArray(item.controls) ? (item.controls as KeyedItem[]) : []) visit(child, path);
  };
  sections.forEach(section => visit(section));
  return paths;
};

/** Drops entries for paths and plugin-owned controls that disappeared with a deleted plugin. */
export const pruneToolbarUsage = (validPaths: ReadonlyMap<string, string | undefined>): void => {
  pruneRequested = true;
  validPathsAfterPrune = validPaths;
  const before = toolbarUsage.value;
  const recent = applyPathPrune(before, validPaths);
  const changed = JSON.stringify(recent) !== JSON.stringify(before);
  if (changed) toolbarUsage.next(recent);
  if (loaded && changed) save(recent);
  else if (loaded && !Object.keys(recent).length) save({});
};

const score = (path: string, usage: ToolbarUsage) => {
  const recent = Object.entries(usage)
    .filter(([itemPath]) => itemPath === path || itemPath.startsWith(`${path}/`))
    .flatMap(([, entry]) => entry.lastUsed)
    .sort()
    .slice(-TOOLBAR_USAGE_WINDOW);
  return { count: recent.length, latest: recent[recent.length - 1] ?? '' };
};

/** Sorts tools and categories by recent use, preserving declared order for ties and unused items. */
export const orderByToolbarUse = <T extends KeyedItem>(items: readonly T[], usage: ToolbarUsage, parentPath: string): T[] =>
  items
    .map((item, originalIndex) => ({ item, originalIndex, ...score(itemPath(parentPath, item), usage) }))
    .sort((a, b) => b.count - a.count || b.latest.localeCompare(a.latest) || a.originalIndex - b.originalIndex)
    .map(({ item }) => item);

/** Orders section tabs using their separate recent-use histories. */
export const orderToolbarSections = <T extends KeyedItem>(sections: readonly T[], usage: ToolbarUsage): T[] =>
  sections
    .map((section, originalIndex) => ({ section, originalIndex, ...score(pathSlug(section.label ?? section.key), usage) }))
    .sort((a, b) => b.count - a.count || b.latest.localeCompare(a.latest) || a.originalIndex - b.originalIndex)
    .map(({ section }) => section);

/** The readable path of a section or control, using the same labels as the editor shows. */
export const toolbarPath = (...labels: string[]): string => labels.map(pathSlug).filter(Boolean).join('/');
