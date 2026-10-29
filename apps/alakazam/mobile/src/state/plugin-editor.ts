import type { PickedImage } from '@/src/components/file-picker';
import type { PluginInspectorItem } from '@/src/lib/plugins/plugin-inspector';
import type { CanvasLike } from '@abra/core/canvas';
import { BehaviorSubject, Subject } from 'rxjs';

/** Whether the inspector is open over the preview, on a screen too narrow to show it beside the preview. */
export const pluginEditorInspectorOpen = new BehaviorSubject(false);
export const pluginEditorInspectorOpen$ = pluginEditorInspectorOpen.asObservable();

/** Whether the open workspace has an inspector at all: its script exports `inspectorGUI`, or failed trying to. */
export const pluginEditorHasInspector = new BehaviorSubject(false);
export const pluginEditorHasInspector$ = pluginEditorHasInspector.asObservable();

/** Whether the open workspace has a controls bar: its script exports `controlsGUI`, or failed trying to. */
export const pluginEditorHasControls = new BehaviorSubject(false);
export const pluginEditorHasControls$ = pluginEditorHasControls.asObservable();

/** The controls the workspace's `inspectorGUI` and `controlsGUI` made the last time they ran, each with the `surface` it is for. */
export const pluginEditorItems = new BehaviorSubject<PluginInspectorItem[]>([]);
export const pluginEditorItems$ = pluginEditorItems.asObservable();

/** Why the inspector could not be made, in words for a person. `null` when it was. */
export const pluginEditorInspectorError = new BehaviorSubject<string | null>(null);
export const pluginEditorInspectorError$ = pluginEditorInspectorError.asObservable();

/** Why the controls bar could not be made, in words for a person. `null` when it was. */
export const pluginEditorControlsError = new BehaviorSubject<string | null>(null);
export const pluginEditorControlsError$ = pluginEditorControlsError.asObservable();

/** The canvas the workspace's `previewCanvas` returned the last time it ran. `null` before the first, or when it returned none. The
 * plugin owns it; the preview only shows it. */
export const pluginEditorCanvas = new BehaviorSubject<CanvasLike | null>(null);
export const pluginEditorCanvas$ = pluginEditorCanvas.asObservable();

/** Why `previewCanvas` could not make its canvas, in words for a person. `null` when it did. */
export const pluginEditorPreviewError = new BehaviorSubject<string | null>(null);
export const pluginEditorPreviewError$ = pluginEditorPreviewError.asObservable();

/** Whether `previewCanvas` is still making a canvas, which the preview shows with a spinner over the canvas it has until the new one arrives. */
export const pluginEditorBuilding = new BehaviorSubject(false);
export const pluginEditorBuilding$ = pluginEditorBuilding.asObservable();

/** The images each file picker holds, by the picker's `key`. */
export const pluginEditorPicked = new BehaviorSubject<Record<string, PickedImage[]>>({});
export const pluginEditorPicked$ = pluginEditorPicked.asObservable();

/** Said when a control changed something that needs the plugin run again (a size typed in), as opposed to a drag, which only moves what
 * the canvas already has. The open workspace runs its controls and `previewCanvas` again. */
export const pluginEditorRebuild = new Subject<void>();
export const pluginEditorRebuild$ = pluginEditorRebuild.asObservable();

/** Said each time a control changes something the preview shows without a new canvas (a slider moved the layers of the one it has).
 * The preview audits it, so a drag draws once a frame and the slider is drawn first. */
export const pluginEditorRedraw = new Subject<void>();
export const pluginEditorRedraw$ = pluginEditorRedraw.asObservable();

/** Said after a development plugin is installed, so an open workspace for that plugin can reload its script. */
export const pluginEditorDevUpdate = new Subject<string>();
export const pluginEditorDevUpdate$ = pluginEditorDevUpdate.asObservable();
