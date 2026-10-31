import { Subscription } from 'rxjs';
import { pluginsFolder, readPluginScript, scanPlugins, type PluginInfo } from '@alakazam/mobile';
import { EDIT_SECTIONS, leafControls, type EditControl } from '@/src/lib/project/edit-sections';
import { insertControls, removeControls } from '@/src/lib/plugins/control-insertion';
import type { ControlScript } from '@/src/lib/plugins/sdk-runtime';
import { pluginModules } from '@/src/lib/plugins/plugin-host';
import { runPluginScript } from '@/src/lib/plugins/plugin-script';
import { resolveRoot, type Place, type PlaceMaker } from '@/src/lib/plugins/root-path';
import { errorMessage } from '@/src/lib/utils/error-message';
import { clearControlTraits, setControlTraits } from '@/src/state/control-traits';
import { pluginControlsVersion, pluginProblems, type PluginProblem } from '@/src/state/plugins';
import { objectRemovalToolKey } from '@/src/state/plugin-tool-panels';
import { removePluginInspectorItems } from '@/src/lib/plugins/plugin-inspector';

/** What a plugin put in the editor, and how to take it out again. */
type Loaded = { version: string; undo: () => void };

/**
 * The plugins whose controls are in the editor now, by id. A change to this file while the app runs (Fast Refresh) starts it empty
 * again, so a control is put in only after one with its key is taken out: it never shows twice.
 */
const loaded = new Map<string, Loaded>();
const legacyBeautyKeys = new Set([
  'group-skin', 'group-hair', 'action-skin-even-out', 'action-skin-tan', 'action-skin-tone',
  'action-skin-smooth', 'slider-teeth-whitening', 'action-red-eye', 'action-blemish', 'slider-hair-color',
]);

const slug = (label: string) =>
  label
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '');

/** Retains the owner on every contributed control so saved usage can be removed with its plugin. */
function markPluginOwner(control: EditControl, pluginId: string): void {
  control.pluginId = pluginId;
  if ('controls' in control) for (const child of control.controls) markPluginOwner(child, pluginId);
}

/** Plugin control keys are persisted in editor state, so keep each one namespaced to its author and plugin. */
function validateControlKeys(plugin: PluginInfo, control: EditControl): void {
  const prefix = `${plugin.id}.`;
  const suffix = control.key.startsWith(prefix) ? control.key.slice(prefix.length) : '';
  const legacy = plugin.id === 'alakazam.beauty' && legacyBeautyKeys.has(control.key);
  if (!legacy && !/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(suffix)) {
    throw new Error(`Control key "${control.key}" must follow "${plugin.id}.<tool>" (for example, "${plugin.id}.remove-objects").`);
  }
  if ('controls' in control) for (const child of control.controls) validateControlKeys(plugin, child);
}

/** Removes controls left by an earlier load of this plugin, including controls with keys it has since renamed. */
function removePluginControls(controls: EditControl[], prefix: string): void {
  for (let index = controls.length - 1; index >= 0; index--) {
    const control = controls[index];
    if (control.key.startsWith(prefix)) {
      for (const leaf of leafControls([control])) if (leaf.kind === 'slider') clearControlTraits(leaf.key);
      controls.splice(index, 1);
    } else if ('controls' in control) {
      removePluginControls(control.controls, prefix);
    }
  }
}

/** Removes a control key left in memory by an older version of the app. */
function removeControlByKey(controls: EditControl[], key: string): void {
  for (let index = controls.length - 1; index >= 0; index--) {
    const control = controls[index];
    if (control.key === key) controls.splice(index, 1);
    else if ('controls' in control) removeControlByKey(control.controls, key);
  }
}

/** A tab or group that a plugin's root names and the editor does not have yet. */
const maker: PlaceMaker = {
  section: label => ({ key: label.trim().toLowerCase() === 'beauty' ? 'section-beauty' : `section-plugin-${slug(label)}`, label, controls: [] }),
  group: (label, parentKey) => ({ kind: 'group', key: `group-plugin-${parentKey}-${slug(label)}`, label, controls: [] }),
};

/** Puts the controls of one plugin in the editor, each where its `plugin.yml` says. Puts nothing in when any of them fails. */
function loadControls(plugin: PluginInfo): Loaded {
  const undos: (() => void)[] = [];
  // What the plugin's controls listen to through `events` ends when they are taken out.
  const events = new Subscription();
  undos.push(() => events.unsubscribe());
  removePluginInspectorItems(plugin.id);
  try {
    // Plugin controls may outlive this module during Fast Refresh. Clear the whole plugin namespace before loading the current version.
    for (const section of EDIT_SECTIONS) removePluginControls(section.controls, `${plugin.id}.`);
    // Older app builds supplied object removal as a built-in Cleanup item. It must not remain beside the plugin-owned control.
    if (plugin.id === 'alakazam.object-remover') {
      for (const section of EDIT_SECTIONS) removeControlByKey(section.controls, 'action-object-removal');
    }
    for (const control of plugin.controls) {
      const names = control.path.split('/');
      const name = names[names.length - 1];
      // The controls go under the root, and under the groups of their own path, if the path has any.
      const root = [control.root, ...names.slice(0, -1)].join('/');
      const what = `${plugin.name}: ${control.path}`;
      const made = runPluginScript(readPluginScript(control.entryPath), pluginModules(plugin.id, events), what).default;
      if (typeof made !== 'function') throw new Error(`${what} has no default export that is a function.`);
      // The plugin's yml says what the control is called, so the label people see is the name there.
      const placed = { ...(made as ControlScript)(), label: name } as EditControl;
      markPluginOwner(placed, plugin.id);
      validateControlKeys(plugin, placed);

      const { list, undo: undoPlaces } = resolveRoot(EDIT_SECTIONS as unknown as Place[], root, maker);
      const controls = list as unknown as EditControl[];
      removeControls(
        controls,
        controls.filter(other => other.key === placed.key),
      );
      insertControls(controls, [placed]);
      if (placed.kind === 'tool' && placed.panel === 'object-removal') objectRemovalToolKey.next(placed.key);
      for (const leaf of leafControls([placed])) if (leaf.kind === 'slider') setControlTraits(leaf.key, leaf);
      undos.push(() => {
        removeControls(controls, [placed]);
        if (placed.kind === 'tool' && placed.panel === 'object-removal' && objectRemovalToolKey.value === placed.key) {
          objectRemovalToolKey.next(null);
        }
        for (const leaf of leafControls([placed])) if (leaf.kind === 'slider') clearControlTraits(leaf.key);
        undoPlaces();
      });
    }
  } catch (error) {
    undos.reverse().forEach(undo => undo());
    removePluginInspectorItems(plugin.id);
    throw error;
  }
  return {
    version: plugin.version,
    undo: () => {
      [...undos].reverse().forEach(undo => undo());
      removePluginInspectorItems(plugin.id);
    },
  };
}

/**
 * Brings the controls in the editor in line with the plugins installed on this device: a plugin that is installed, can be used, and adds
 * controls has them in the editor, one that was taken away or changed version has them out (or put in again). A plugin that fails to load
 * is reported in `pluginProblems` and does not stop the others. Cheap when nothing changed, so it can be called whenever a plugin is
 * installed, replaced or deleted, and when the app starts.
 */
export function loadPlugins(force = false): void {
  const scan = scanPlugins(pluginsFolder());
  const problems: PluginProblem[] = scan.problems.map(problem => ({ plugin: problem.dir, message: problem.message }));
  const hasRenamedBeauty = scan.plugins.some(plugin => plugin.id === 'alakazam.beauty');

  const wanted = new Map<string, PluginInfo>();
  for (const plugin of scan.plugins) {
    // A changed plugin ID installs alongside the old package. Prefer Beauty when both IDs are present to avoid duplicate controls.
    if (hasRenamedBeauty && plugin.id === 'alakazam.makeup') continue;
    if (plugin.controls.length === 0) continue;
    if (plugin.missingDependencies.length > 0) {
      problems.push({ plugin: plugin.name, message: `Needs ${plugin.missingDependencies.join(', ')} installed first.` });
      continue;
    }
    wanted.set(plugin.id, plugin);
  }

  let changed = false;
  for (const [id, was] of loaded) {
    if (!force && wanted.get(id)?.version === was.version) continue;
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

/** Runs the plugin-owned callback for a selected custom tool, passing its host-collected interaction value. */
export function applyPluginTool(key: string, value?: unknown): { handled: boolean; result?: unknown } {
  for (const section of EDIT_SECTIONS) {
    const control = leafControls(section.controls).find(each => each.kind === 'tool' && each.key === key);
    if (control?.kind === 'tool' && control.onApply) {
      return { handled: true, result: control.onApply(value) };
    }
  }
  return { handled: false };
}
