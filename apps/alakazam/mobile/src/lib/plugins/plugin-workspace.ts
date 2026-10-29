import type { PickedImage } from '@/src/components/file-picker';
import { pluginModules } from '@/src/lib/plugins/plugin-host';
import {
  createPluginFilePickerItem,
  createPluginHeaderItem,
  createPluginSliderItem,
  createPluginNumberItem,
  createPluginSelectItem,
  createPluginSizeItem,
  createPluginToggleItem,
  removePluginInspectorItems,
  type PluginInspectorItem,
} from '@/src/lib/plugins/plugin-inspector';
import { runPluginScript } from '@/src/lib/plugins/plugin-script';
import {
  pluginEditorBuilding,
  pluginEditorCanvas,
  pluginEditorControlsError,
  pluginEditorHasControls,
  pluginEditorHasInspector,
  pluginEditorInspectorError,
  pluginEditorInspectorOpen,
  pluginEditorItems,
  pluginEditorPicked,
  pluginEditorPreviewError,
  pluginEditorRebuild$,
  pluginEditorRedraw,
} from '@/src/state/plugin-editor';
import type { CanvasLike } from '@abra/core/canvas';
import { readPluginScript } from '@alakazam/mobile';

/** A workspace a plugin adds, as the plugins screen lists it. */
export type PluginWorkspaceInfo = { pluginId: string; id: string; title: string; entryPath: string };

/** What the open workspace's script exports. Every export is optional, and a workspace shows only the parts it exports. */
type WorkspaceModule = {
  /** Adds the controls of the inspector beside the preview. */
  inspectorGUI?: () => void;
  /** Adds the controls of the bar under the preview. Made the way `inspectorGUI` is. */
  controlsGUI?: () => void;
  /** Returns the canvas the preview shows, or nothing, or a promise of it, for a canvas that takes a while to make (build it with the
   * library's workers, such as `canvasOfCells`, and not with a loop on the JavaScript thread). Called when the workspace opens and after
   * each change that is not a drag; the canvas the plugin returns again is the one already on show, with whatever the plugin changed in
   * it. While a promise is pending the canvas on show stays, with a spinner over it, and only the newest call's canvas is shown. The
   * plugin owns every canvas it makes, including one that arrives after a newer call or after the workspace closed. */
  previewCanvas?: () => CanvasLike | null | undefined | Promise<CanvasLike | null | undefined>;
  /** Called when the workspace closes, so the plugin can free what it made. */
  dispose?: () => void;
};

/** The surfaces a workspace's controls are made for, and the export that makes the controls of each. */
const SURFACES = [
  { surface: 'inspector', run: 'inspectorGUI', has: pluginEditorHasInspector, error: pluginEditorInspectorError },
  { surface: 'controls', run: 'controlsGUI', has: pluginEditorHasControls, error: pluginEditorControlsError },
] as const;

/** What is open, so a change to a control can run the plugin again. `null` when no workspace is open. */
let open: { rebuild: () => void } | null = null;

const message = (error: unknown) => String((error as Error)?.message ?? error);

/**
 * Opens a workspace: runs its script, lets it make its controls and its canvas, and puts what it made in the plugin editor state, which
 * the screen only shows. Returns the function that closes it, which tells the plugin and clears the state.
 */
export function openPluginWorkspace(workspace: PluginWorkspaceInfo): () => void {
  pluginEditorInspectorOpen.next(false);
  pluginEditorHasInspector.next(false);
  pluginEditorHasControls.next(false);
  pluginEditorItems.next([]);
  pluginEditorInspectorError.next(null);
  pluginEditorControlsError.next(null);
  pluginEditorCanvas.next(null);
  pluginEditorPreviewError.next(null);
  pluginEditorBuilding.next(false);
  pluginEditorPicked.next({});
  removePluginInspectorItems(workspace.pluginId);
  open = null;
  let module: WorkspaceModule | null = null;
  // A control that needs the plugin run again (a size typed in) says so here.
  const rebuilds = pluginEditorRebuild$.subscribe(() => open?.rebuild());
  try {
    // The controls a plugin adds while one of its exports runs are collected here, for the surface that export is for.
    let collecting: PluginInspectorItem[] | null = null;
    module = runPluginScript(
      readPluginScript(workspace.entryPath),
      pluginModules(workspace.pluginId, {
        filePicker: (label, options) => collecting?.push(createPluginFilePickerItem(workspace.pluginId, label, options)),
        header: (text, options) => collecting?.push(createPluginHeaderItem(workspace.pluginId, text, options)),
        slider: (label, options) => collecting?.push(createPluginSliderItem(workspace.pluginId, label, options)),
        size: (label, options) => collecting?.push(createPluginSizeItem(workspace.pluginId, label, options)),
        select: (label, options) => collecting?.push(createPluginSelectItem(workspace.pluginId, label, options)),
        number: (label, options) => collecting?.push(createPluginNumberItem(workspace.pluginId, label, options)),
        toggle: (label, options) => collecting?.push(createPluginToggleItem(workspace.pluginId, label, options)),
      }),
      `${workspace.title} workspace`,
    ) as WorkspaceModule;
    const script = module;

    /** Runs one export that makes controls, and puts what it made in the state in place of that surface's last controls. */
    const makeControls = ({ surface, run, error }: (typeof SURFACES)[number]) => {
      const render = script[run];
      if (typeof render !== 'function') return;
      collecting = [];
      try {
        render();
        error.next(null);
      } catch (failure) {
        error.next(message(failure));
      } finally {
        // Two controls can share a label, so each is named by its surface, its kind, its label and how many came before it with them.
        const seen = new Map<string, number>();
        const made = (collecting ?? []).map(item => {
          const name = `${surface}:${item.kind}:${item.label}`;
          const count = seen.get(name) ?? 0;
          seen.set(name, count + 1);
          return { ...item, surface, key: `${name}:${count}` } as PluginInspectorItem;
        });
        collecting = null;
        pluginEditorItems.next([...pluginEditorItems.value.filter(item => item.surface !== surface), ...made]);
      }
    };

    // Each call is numbered, and only the newest one's canvas is shown: a call that was slow and has been replaced is out of date.
    let previewRun = 0;
    const makePreview = async () => {
      if (typeof script.previewCanvas !== 'function') return;
      const run = ++previewRun;
      pluginEditorBuilding.next(true);
      try {
        const made = await script.previewCanvas();
        if (run !== previewRun || open === null) return;
        pluginEditorCanvas.next(made ?? null);
        pluginEditorPreviewError.next(null);
        // The plugin may have changed the canvas it returned again, so the preview looks at it again.
        pluginEditorRedraw.next();
      } catch (failure) {
        if (run === previewRun) pluginEditorPreviewError.next(message(failure));
      } finally {
        if (run === previewRun) pluginEditorBuilding.next(false);
      }
    };

    open = {
      rebuild: () => {
        for (const surface of SURFACES) if (typeof script[surface.run] === 'function') makeControls(surface);
        void makePreview();
      },
    };
    for (const surface of SURFACES) surface.has.next(typeof script[surface.run] === 'function');
    open.rebuild();
  } catch (failure) {
    console.warn(`Could not open the ${workspace.title} workspace:`, failure);
    // A workspace that fails to load still opens its inspector, which says why.
    pluginEditorInspectorError.next(message(failure));
    pluginEditorHasInspector.next(true);
  }
  return () => {
    rebuilds.unsubscribe();
    open = null;
    try {
      module?.dispose?.();
    } catch (failure) {
      console.warn(`The ${workspace.title} workspace could not clean up:`, failure);
    }
    removePluginInspectorItems(workspace.pluginId);
    pluginEditorItems.next([]);
    pluginEditorCanvas.next(null);
  };
}

/** A file picker's images changed, whether images were picked or removed. Keeps the list, tells the plugin the whole list, then runs the plugin again. */
export function updatePluginFiles(item: Extract<PluginInspectorItem, { kind: 'file-picker' }>, files: PickedImage[]) {
  pluginEditorPicked.next({ ...pluginEditorPicked.value, [item.key]: files });
  try {
    item.onPick(files);
  } finally {
    open?.rebuild();
  }
}
