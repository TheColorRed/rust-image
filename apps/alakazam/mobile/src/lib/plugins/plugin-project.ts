import type { PickedImage } from '@/src/components/file-picker';
import type { PluginInspectorItem } from '@/src/lib/plugins/plugin-inspector';
import { addProjectFile, createPluginProject, type Project } from '@/src/lib/project/project-store';
import { toLocalPath } from '@/src/lib/editor/edit-source';
import { release } from '@/src/lib/native/native-handles';
import {
  pluginEditorCanvas,
  pluginEditorItems,
  pluginEditorItems$,
  pluginEditorPicked,
  pluginEditorPicked$,
  pluginEditorProjectId,
  pluginEditorRedraw$,
  pluginEditorSaved,
  pluginEditorSaved$,
} from '@/src/state/plugin-editor';
import { addPluginProject, projects, updateProject } from '@/src/state/projects';
import type { CanvasLike } from '@abra/core/canvas';
import { combineLatest, debounceTime, startWith } from 'rxjs';

/** The project the open workspace saves into. Empty until the user first picks a picture, so a workspace that is only looked at leaves nothing behind. */
export type PluginProjectRef = { current: Project | undefined };

/** The pause after the last change before the project is written, so a run of changes is one write. */
const SAVE_DELAY_MS = 800;

/** The quality of the picture of the whole result that the home grid shows. */
const RESULT_QUALITY = 85;

/** What a control holds that is worth saving, or `undefined` for one that holds nothing (a heading, a file picker). */
const valueOf = (item: PluginInspectorItem): unknown =>
  item.kind === 'header' || item.kind === 'file-picker' || item.kind === 'pictures' || (item.kind === 'select' && item.momentary)
    ? undefined
    : item.value;

/** Every control's value and every picker's pictures, in one string, to tell whether anything changed since the last write. */
const snapshotOf = (
  items: PluginInspectorItem[],
  picked: Record<string, PickedImage[]>,
  kept: Record<string, unknown>,
  drawn: number,
) =>
  JSON.stringify({
    drawn,
    kept,
    values: items.flatMap(item => (valueOf(item) === undefined ? [] : [[item.key, valueOf(item)]])),
    pickers: Object.entries(picked).map(([key, files]) => [key, files.map(file => file.uri)]),
  });

/**
 * Puts a saved project's pictures and control values back into the workspace that just opened. A control can make other controls (the
 * style picks which size fields show), so it applies what the controls there are, lets the plugin run again, and repeats for the new ones.
 */
export function restorePluginProject(project: Project, rebuild: () => void): void {
  const saved = project.plugin;
  if (!saved) return;
  const applied = new Set<string>();
  for (let round = 0; round < 4; round++) {
    const fresh = pluginEditorItems.value.filter(
      // A row of buttons holds no value: a project saved before that was known has its first button saved, and pressing it would act.
      item =>
        !applied.has(item.key) &&
        !(item.kind === 'select' && item.momentary) &&
        (item.key in saved.values || item.key in saved.pickers),
    );
    if (fresh.length === 0) return;
    for (const item of fresh) {
      applied.add(item.key);
      if (item.kind === 'file-picker') {
        const files = saved.pickers[item.key].map(({ name, file }) => ({ uri: `${project.dir}/${file}`, name, fileCopyUri: `${project.dir}/${file}` }));
        pluginEditorPicked.next({ ...pluginEditorPicked.value, [item.key]: files });
        item.onPick(files);
      } else if (item.kind === 'font-picker') {
        item.onPick?.(saved.values[item.key] as string);
      } else if (item.kind !== 'header') {
        (item.onChange as ((value: unknown) => void) | undefined)?.(saved.values[item.key]);
      }
    }
    rebuild();
  }
}

/**
 * Saves the open workspace into its project each time its controls or pictures change and then stop changing. The project is made when the
 * first picture is picked. Each picture is copied into the project folder once, so the project does not depend on the cache. Returns the
 * function that stops it, which writes any change still waiting first.
 */
export function autosavePluginProject(
  workspace: { pluginId: string; id: string; title: string },
  ref: PluginProjectRef,
): () => void {
  // What was last written, so a rebuild that changed nothing writes nothing.
  let written = snapshotOf(pluginEditorItems.value, pluginEditorPicked.value, pluginEditorSaved.value, 0);
  // Pictures already copied into the project, by the cache file they came from.
  const copied = new Map<string, string>();
  let queue = Promise.resolve();

  const save = async (
    items: PluginInspectorItem[],
    picked: Record<string, PickedImage[]>,
    kept: Record<string, unknown>,
    canvas: CanvasLike | null,
    drawn: number,
  ) => {
    // No controls means the workspace is still opening or has been cleared, and saving that would erase what the project holds.
    if (items.length === 0) return;
    const snapshot = snapshotOf(items, picked, kept, drawn);
    if (snapshot === written) return;
    const hasPictures = Object.values(picked).some(files => files.length > 0);
    if (!ref.current && !hasPictures) return;

    // The project is in the list once it has been written with a picture. Until then a failed save is tried again on the same project, not a new one.
    const project = ref.current ?? (await createPluginProject(workspace.title, workspace.pluginId, workspace.id));
    ref.current = project;
    pluginEditorProjectId.next(project.id);
    const isNew = !projects.value.some(listed => listed.id === project.id);
    const pickers: NonNullable<Project['plugin']>['pickers'] = {};
    for (const [key, files] of Object.entries(picked)) {
      pickers[key] = [];
      for (const file of files) {
        const inside = file.uri.startsWith(`${project.dir}/`);
        let stored = inside ? file.uri.slice(project.dir.length + 1) : copied.get(file.uri);
        if (!stored) {
          // Named after the file itself, not the picture's display name: the extension has to match what is inside, or it fails to decode.
          stored = await addProjectFile(project.id, file.uri, decodeURIComponent(file.uri.split('/').pop() ?? 'image'));
          copied.set(file.uri, stored);
        }
        pickers[key].push({ name: file.name, file: stored });
      }
    }
    // The controls on show are only some of the plugin's (a sub-inspector hides the rest), so what the project held for the others stays.
    const values = {
      ...project.plugin?.values,
      ...Object.fromEntries(items.flatMap(item => (valueOf(item) === undefined ? [] : [[item.key, valueOf(item)]]))),
    };
    // The home grid shows the whole result. If the canvas is not made yet or cannot be written, it shows the first picture.
    let resultFile = Object.values(pickers).flat()[0]?.file ?? project.resultFile;
    if (canvas) {
      const name = `result-${Date.now()}.webp`;
      const image = canvas.asImage();
      try {
        image.write(toLocalPath(`${project.dir}/${name}`), { quality: RESULT_QUALITY });
        resultFile = name;
      } catch (error) {
        console.warn('Could not save the plugin project picture:', error);
      } finally {
        release(image);
      }
    }
    const saved: Project = {
      ...project,
      resultFile,
      plugin: { pluginId: workspace.pluginId, workspaceId: workspace.id, values, pickers, state: kept },
    };
    // A project with no picture to show is not put in the grid.
    if (!saved.resultFile) return;
    ref.current = saved;
    if (isNew) await addPluginProject(saved);
    else await updateProject(saved);
    written = snapshot;
  };

  // Counts how many times the preview was drawn, so a canvas that changed with nothing else changing is still written.
  let drawn = 0;
  const redraws = pluginEditorRedraw$.subscribe(() => drawn++);
  const subscription = combineLatest([
    pluginEditorItems$,
    pluginEditorPicked$,
    pluginEditorSaved$,
    pluginEditorRedraw$.pipe(startWith(undefined)),
  ])
    .pipe(debounceTime(SAVE_DELAY_MS))
    .subscribe(([items, picked, kept]) => {
      const [canvas, count] = [pluginEditorCanvas.value, drawn];
      queue = queue.then(() => save(items, picked, kept, canvas, count)).catch(error => console.warn('Could not save the plugin project:', error));
    });

  return () => {
    redraws.unsubscribe();
    subscription.unsubscribe();
    // The workspace is closing and is about to clear its state, so what the user did in the last moments is written from what is there now.
    const [items, picked, kept, canvas, count] = [
      pluginEditorItems.value,
      pluginEditorPicked.value,
      pluginEditorSaved.value,
      pluginEditorCanvas.value,
      drawn,
    ];
    void queue.then(() => save(items, picked, kept, canvas, count)).catch(error => console.warn('Could not save the plugin project:', error));
  };
}
