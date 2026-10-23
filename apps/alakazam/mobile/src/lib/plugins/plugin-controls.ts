import { scanPlugins, type PluginInfo } from '@abra/core';
import { pluginsFolder, readPluginScript } from '@alakazam/mobile';
import { EDIT_SECTIONS, leafControls, type EditControl } from '@/src/lib/project/edit-sections';
import { insertControls, removeControls } from '@/src/lib/plugins/control-insertion';
import type { ControlScript } from '@/src/lib/plugins/host-api';
import { PLUGIN_MODULES } from '@/src/lib/plugins/plugin-host';
import { runPluginScript } from '@/src/lib/plugins/plugin-script';
import { resolveRoot, type Place, type PlaceMaker } from '@/src/lib/plugins/root-path';
import { errorMessage } from '@/src/lib/utils/error-message';
import { clearControlTraits, setControlTraits } from '@/src/state/control-traits';
import { pluginControlsVersion, pluginProblems, type PluginProblem } from '@/src/state/plugins';

/** What a plugin put in the editor, and how to take it out again. */
type Loaded = { version: string; undo: () => void };

/**
 * The plugins whose controls are in the editor now, by id. A change to this file while the app runs (Fast Refresh) starts it empty
 * again, so a control is put in only after one with its key is taken out: it never shows twice.
 */
const loaded = new Map<string, Loaded>();

const slug = (label: string) =>
  label
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '');

/** A tab or group that a plugin's root names and the editor does not have yet. */
const maker: PlaceMaker = {
  section: label => ({ key: `section-plugin-${slug(label)}`, label, controls: [] }),
  group: (label, parentKey) => ({ kind: 'group', key: `group-plugin-${parentKey}-${slug(label)}`, label, controls: [] }),
};

/** Puts the controls of one plugin in the editor, each where its `plugin.yml` says. Puts nothing in when any of them fails. */
function loadControls(plugin: PluginInfo): Loaded {
  const undos: (() => void)[] = [];
  try {
    for (const control of plugin.controls) {
      const names = control.path.split('/');
      const name = names[names.length - 1];
      // The controls go under the root, and under the groups of their own path, if the path has any.
      const root = [control.root, ...names.slice(0, -1)].join('/');
      const what = `${plugin.name}: ${control.path}`;
      const made = runPluginScript(readPluginScript(control.entryPath), PLUGIN_MODULES, what).default;
      if (typeof made !== 'function') throw new Error(`${what} has no default export that is a function.`);
      // The plugin's yml says what the control is called, so the label people see is the name there.
      const placed = { ...(made as ControlScript)(), label: name } as EditControl;

      const { list, undo: undoPlaces } = resolveRoot(EDIT_SECTIONS as unknown as Place[], root, maker);
      const controls = list as unknown as EditControl[];
      removeControls(
        controls,
        controls.filter(other => other.key === placed.key),
      );
      insertControls(controls, [placed]);
      for (const leaf of leafControls([placed])) if (leaf.kind === 'slider') setControlTraits(leaf.key, leaf);
      undos.push(() => {
        removeControls(controls, [placed]);
        for (const leaf of leafControls([placed])) if (leaf.kind === 'slider') clearControlTraits(leaf.key);
        undoPlaces();
      });
    }
  } catch (error) {
    undos.reverse().forEach(undo => undo());
    throw error;
  }
  return { version: plugin.version, undo: () => [...undos].reverse().forEach(undo => undo()) };
}

/**
 * Brings the controls in the editor in line with the plugins installed on this device: a plugin that is installed, can be used, and adds
 * controls has them in the editor, one that was taken away or changed version has them out (or put in again). A plugin that fails to load
 * is reported in `pluginProblems` and does not stop the others. Cheap when nothing changed, so it can be called whenever a plugin is
 * installed, replaced or deleted, and when the app starts.
 */
export function loadPlugins(): void {
  const scan = scanPlugins(pluginsFolder());
  const problems: PluginProblem[] = scan.problems.map(problem => ({ plugin: problem.dir, message: problem.message }));

  const wanted = new Map<string, PluginInfo>();
  for (const plugin of scan.plugins) {
    if (plugin.controls.length === 0) continue;
    if (plugin.missingDependencies.length > 0) {
      problems.push({ plugin: plugin.name, message: `Needs ${plugin.missingDependencies.join(', ')} installed first.` });
      continue;
    }
    wanted.set(plugin.id, plugin);
  }

  let changed = false;
  for (const [id, was] of loaded) {
    if (wanted.get(id)?.version === was.version) continue;
    was.undo();
    loaded.delete(id);
    changed = true;
  }
  for (const plugin of wanted.values()) {
    if (loaded.has(plugin.id)) continue;
    try {
      loaded.set(plugin.id, loadControls(plugin));
      changed = true;
    } catch (error) {
      problems.push({ plugin: plugin.name, message: errorMessage(error) });
    }
  }

  pluginProblems.next(problems);
  if (changed) pluginControlsVersion.next(pluginControlsVersion.value + 1);
}
