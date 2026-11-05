/**
 * Runtime values the app offers a plugin script as the module `@alakazam/sdk`. The public types live in the workspace npm package
 * so plugin authors can install them without the app source.
 *
 * Keep it small and stable. Anything added here is something every plugin can depend on.
 */
import { map } from 'rxjs/operators';
import { Observable } from 'rxjs';
import { computePreviewImagePoint } from '@/src/lib/editor/preview-coordinates';
import { pickColor } from '@/src/state/color-picker';
import { previewBox, previewOrigin, previewPressed$, previewRotationValue, previewScaleValue, previewTranslateValue } from '@/src/state/gestures';
import { pluginEditorCanvas, pluginEditorInspectorOpen, pluginEditorRebuild, pluginEditorRedraw, pluginEditorSaved } from '@/src/state/plugin-editor';
import { editorGizmos } from '@/src/state/gizmos';
import { pluginPinch$, pluginPinchClaim, pluginPinchListeners, pluginTouch$, pluginTouchListeners, type PluginPinch, type PluginTouch } from '@/src/state/plugin-touch';
import { missingModels } from '@/src/lib/ai/ai-model-availability';
import { framedPeople } from '@/src/lib/ai/analysis';
import { aiModelLifetimes, registerAiModel, releaseAiModel, runWithAiModel } from '@/src/lib/ai/model-warmth';
import { commitRetouch } from '@/src/lib/editor/editor-history';
import { flushFullReplay } from '@/src/lib/editor/editor-replay';
import { saveOpenProject } from '@/src/lib/editor/editor-session';
import { installedFamilies, renderText } from '@/src/lib/fonts/text-image';
import { release } from '@/src/lib/native/native-handles';
import {
  addPluginFilePicker,
  addPluginFontPicker,
  addPluginCurve,
  addPluginGradient,
  addPluginGroup,
  addPluginHeader,
  addPluginSubmenu,
  type PluginSubmenuOptions,
  type PluginCurveOptions,
  type PluginGradientOptions,
  type PluginGroupOptions,
  addPluginNumber,
  addPluginSelect,
  addPluginSize,
  addPluginSlider,
  addPluginText,
  type PluginFilePickerOptions,
  type PluginFontPickerOptions,
  type PluginPicturesOptions,
  addPluginPictures,
  type PluginHeaderOptions,
  type PluginNumberOptions,
  type PluginSelectOptions,
  type PluginSizeOptions,
  type PluginSliderOptions,
  type PluginTextOptions,
} from '@/src/lib/plugins/plugin-inspector';
import { aiModelList, aiModelsRequested } from '@/src/state/ai-models';
import { aiProgress, aiProgress$, setAiPendingNotice, type AiPendingNoticeText } from '@/src/state/ai-progress';
import { people$ } from '@/src/state/analysis';
import { adjustments$ } from '@/src/state/edits';
import { timeline$ } from '@/src/state/history';
import { areaRings, areas, areas$, setArea } from '@/src/state/draw-area';
import { historyReady, historyReady$ } from '@/src/state/project-session';
import { busy, currentImage } from '@/src/state/session';
import { setToolAiModel, toolAiModelSelections } from '@/src/state/tool-ai-models';
import { setToolMessage } from '@/src/state/tool-messages';
import { AiModel, type AiModel as AiModelType } from '@abra/core';
import { type CanvasLike, type LayerLike } from '@abra/core/canvas';
import { Color, type ColorLike } from '@abra/core/color';
import { Brush, type BrushLike } from '@abra/core/drawing';
import { type ImageLike } from '@abra/core/image';
import { type MaskLike } from '@abra/core/mask';
import { aiModelPath, runAiModelAsync, runAiModelGuidedAsync, type AbraImageLike } from '@alakazam/mobile';
import Rand from 'rand-seed';
export { belongsTo, findFaces } from '@/src/lib/ai/faces';
export { paintSheet, type Painting } from '@/src/lib/beauty/layers';
export { makeupMasks as faceMasks } from '@/src/lib/beauty/makeup-masks';
export { derivedFor } from '@/src/lib/editor/editor-preloads';
export { areaMask, areaSpots } from '@/src/lib/native/area-mask';
export { owning, release } from '@/src/lib/native/native-handles';
export { swatchesOf } from '@/src/lib/project/edit-sections/swatches';
export { setAiPendingNotice } from '@/src/state/ai-progress';
export type { ControlScript, EditControl, SliderControl, ToolContext } from '@alakazam/sdk';
/**
 * Creates a seeded random number generator using the provided seed string.
 * @param s The seed string used to initialize the random number generator.
 * @returns A seeded random number generator instance.
 */
export const seededRandom = (s: string) => new Rand(s);

/** A whole number from `min` to `max`, either of which it can be, drawn from `rand`. */
export const randomBetween = (rand: Rand, min: number, max: number) => min + Math.floor(rand.next() * (max - min + 1));

/** The numbers 0 up to `count` in an order drawn from `rand`. */
export const shuffled = (count: number, rand: Rand) => {
  const order = Array.from({ length: count }, (_, index) => index);
  for (let at = count - 1; at > 0; at--) {
    const other = randomBetween(rand, 0, at);
    [order[at], order[other]] = [order[other], order[at]];
  }
  return order;
};

/**
 * The width and height of a canvas, or null when there is none or it has been let go. A plugin lets the canvas it made before go as it makes
 * the next, and the app keeps showing the old one until it gets the new, so the canvas on show can be one that is gone.
 */
const canvasSize = (canvas: CanvasLike | null | undefined) => {
  try {
    return canvas ? { width: canvas.width(), height: canvas.height() } : null;
  } catch {
    return null;
  }
};

/** The width and height of the canvas the plugin returned, or null when the preview has none. */
const shownCanvasSize = () => canvasSize(pluginEditorCanvas.value);

/** How many screen pixels across the dot of a point gizmo is, and how much wider its dark halo is. */
const POINT_SIZE = 18;
const POINT_HALO = 8;

/**
 * Draws a dot at `point`, where `[0, 0]` is the top left of the canvas the plugin returned and `[1, 1]` the bottom right. It is a box with
 * no size, so it follows the picture as it is zoomed, moved and turned, and is the same size on the screen at any zoom.
 */
const setPoint = (name: string, point: [number, number], canvas?: CanvasLike) => {
  const gizmos = editorGizmos.value;
  // `previewCanvas` runs before the app shows its canvas, so a plugin names the one it is about to return.
  const size = canvasSize(canvas) ?? shownCanvasSize();
  if (!gizmos || !size) return;
  const at = [point[0] * size.width, point[1] * size.height];
  const corners = [at, at, at, at];
  const [ink, shade, white] = [Color.fromRgba(...(gizmoStyle.rgba as [number, number, number, number])), Color.black(), Color.white()];
  // From the bottom: a dark halo so the dot shows on any picture, a white dot, and the gizmo's color in the middle.
  const layers = [
    { name: `${name}:halo`, brush: brushOf(shade, POINT_SIZE + POINT_HALO, 0.45) },
    { name: `${name}:dot`, brush: brushOf(white, POINT_SIZE, 1) },
    { name, brush: brushOf(ink, POINT_SIZE / 2, 1) },
  ];
  for (const layer of layers) gizmos.setBox(layer.name, corners, layer.brush[layer.brush.length - 1]);
  release(ink, shade, white, ...layers.flatMap(layer => layer.brush));
};

/** Plugin-authored controls that the host renders in the plugin editor inspector. */
export const inspector = {
  filePicker: (label: string, options: PluginFilePickerOptions) => addPluginFilePicker('unscoped', label, options),
  slider: (label: string, options: PluginSliderOptions) => addPluginSlider('unscoped', label, options),
  header: (text: string, options?: PluginHeaderOptions) => addPluginHeader('unscoped', text, options),
  group: (label: string, options?: PluginGroupOptions) => addPluginGroup('unscoped', label, options),
  gradient: (label: string, options?: PluginGradientOptions) => addPluginGradient('unscoped', label, options),
  curve: (label: string, options?: PluginCurveOptions) => addPluginCurve('unscoped', label, options),
  submenu: (label: string, options?: PluginSubmenuOptions) => addPluginSubmenu('unscoped', label, options),
  size: (label: string, options?: PluginSizeOptions) => addPluginSize('unscoped', label, options),
  select: (label: string, options: PluginSelectOptions) => addPluginSelect('unscoped', label, options),
  number: (label: string, options?: PluginNumberOptions) => addPluginNumber('unscoped', label, options),
  /** Makes the inspector, and the preview, run the plugin's controls again, for a change that did not come from one of them (a tap on the preview, say). */
  refresh: () => pluginEditorRebuild.next(),
  /** Says the canvas the plugin returned was changed in place (a layer moved, turned or scaled), so the preview draws it again at once. A slider says this itself; a change that comes from a gesture says it here. */
  redraw: () => pluginEditorRedraw.next(),
  /** Opens the inspector over the preview, on a screen too narrow to show it beside the preview. It does nothing where the inspector is always showing. */
  open: () => pluginEditorInspectorOpen.next(true),
  text: (label: string, options?: PluginTextOptions) => addPluginText('unscoped', label, options),
  fontPicker: (label: string, options?: PluginFontPickerOptions) => addPluginFontPicker('unscoped', label, options),
  pictures: (label: string, options: PluginPicturesOptions) => addPluginPictures('unscoped', label, options),
};

/** Generic app capabilities for plugin controls: editor state, AI models and runs, history, and tool feedback. */
/** Where a point in the preview's pixels is on the open photo, in the photo's pixels, or null for both when it is off the photo. */
const onPhoto = (point: { x: number; y: number }) => {
  const image = currentImage.value as AbraImageLike | null;
  // A plugin's workspace has no open photo: what the preview shows is the canvas the plugin made.
  const canvas = pluginEditorCanvas.value;
  const shown = image ? { width: image.width(), height: image.height() } : canvas ? { width: canvas.width(), height: canvas.height() } : null;
  const box = previewBox.value;
  const at =
    shown && box
      ? computePreviewImagePoint(shown.width, shown.height, box, previewScaleValue.value, previewTranslateValue.value, point, previewRotationValue.value)
      : null;
  return { imageX: at?.x ?? null, imageY: at?.y ?? null };
};

/** The color and width the next gizmos are drawn in (`app.gizmos.color`, `app.gizmos.size`). */
const DEFAULT_GIZMO_RGBA = [255, 255, 255, 255];
const gizmoStyle = { rgba: DEFAULT_GIZMO_RGBA, size: 3 };

/** A brush of this color, width and opacity, made one `with` at a time. Every step is a native object, so all are returned, the last one is the brush, and the caller lets them all go. */
const brushOf = (color: ColorLike, size: number, opacity: number): BrushLike[] => {
  const steps: BrushLike[] = [new Brush()];
  for (const change of [
    (brush: BrushLike) => brush.withColor(color),
    (brush: BrushLike) => brush.withSize(size),
    (brush: BrushLike) => brush.withHardness(1),
    (brush: BrushLike) => brush.withOpacity(opacity),
  ]) {
    steps.push(change(steps[steps.length - 1]));
  }
  return steps;
};

export const app = {
  /** The app's color picker, the same one a plugin's `inspector.color` opens. */
  colors: {
    /**
     * Opens the color picker and answers with the color the person picked when they press OK, as `[red, green, blue]` (each 0 to 255) and the
     * strength as a fourth number when `alpha` is true, or `null` when they cancel.
     */
    pick: async (
      options: {
        title?: string;
        value?: number[];
        alpha?: boolean;
        swatches?: { label: string; rgb: [number, number, number] }[];
        onPreview?: (value: number[]) => void;
      } = {},
    ) => {
      const alpha = options.alpha ?? false;
      const value = options.value ?? [255, 255, 255];
      const picked = await pickColor({
        title: options.title,
        value: [value[0], value[1], value[2]],
        alpha: alpha ? (value[3] ?? 255) : undefined,
        swatches: options.swatches,
        onChange: live => options.onPreview?.(alpha ? [...live.rgb, live.alpha] : [...live.rgb]),
      });
      // Cancelled: the caller takes back what it showed.
      if (!picked) options.onPreview?.([...value]);
      return picked ? (alpha ? [...picked.rgb, picked.alpha] : [...picked.rgb]) : null;
    },
  },
  /**
   * What a plugin keeps with its project, for state that no control holds (which picture a cell was swapped for, say). The values are
   * plain data, saved with the project a moment after they change and given back, before the plugin runs, when the project opens.
   */
  saved: {
    /** What was kept under `name`, or `undefined` when nothing was. */
    get: (name: string): unknown => pluginEditorSaved.value[name],
    /** Keeps `value` under `name`. It must be plain data (numbers, text, lists, objects), and is saved with the project. */
    set: (name: string, value: unknown) => pluginEditorSaved.next({ ...pluginEditorSaved.value, [name]: value }),
  },
  /**
   * Gizmos: shapes the app draws over the preview, which follow the picture as it is zoomed, moved and turned. Positions are in pixels of
   * the canvas the plugin returned. They go away when the workspace closes.
   */
  gizmos: {
    /**
     * Sets the color of the gizmos drawn from now on, until it is set again. Gizmos already drawn keep theirs. `null` goes back to the
     * default, white.
     */
    color: (color: ColorLike | null) => {
      gizmoStyle.rgba = color ? Array.from(new Uint8Array(color.rgba())) : DEFAULT_GIZMO_RGBA;
    },
    /** Sets how wide the lines of the gizmos drawn from now on are, in screen pixels. The default is 3. */
    size: (pixels: number) => {
      gizmoStyle.size = Math.max(1, pixels);
    },
    /**
     * Draws a frame round four corners (each `[x, y]`: top left, top right, bottom right, bottom left) as the gizmo `name`, in the color
     * and width set. A soft dark line is drawn under it so it shows on any picture. Setting a name again moves it.
     */
    setBox: (name: string, corners: number[][]) => {
      const gizmos = editorGizmos.value;
      if (!gizmos) return;
      const [red, green, blue, alpha] = gizmoStyle.rgba;
      const [ink, shade] = [Color.fromRgba(red, green, blue, alpha), Color.black()];
      const [line, edge] = [brushOf(ink, gizmoStyle.size, 1), brushOf(shade, gizmoStyle.size + 6, 0.45)];
      gizmos.setBox(`${name}:edge`, corners, edge[edge.length - 1]);
      gizmos.setBox(name, corners, line[line.length - 1]);
      release(ink, shade, ...line, ...edge);
    },
    setPoint,
    /**
     * Lets a person tap the preview to choose a point. Each tap on the canvas the plugin returned calls `onPick` with the point as shares of
     * its width and height (`[0, 0]` is the top left and `[1, 1]` the bottom right), and puts the dot gizmo `name` there. A tap off the canvas
     * does nothing. Returns what stops it, which also stops when the plugin closes.
     */
    pickPoint: (name: string, onPick: (point: [number, number]) => void) =>
      previewPressed$.pipe(map(tap => onPhoto(tap))).subscribe(({ imageX, imageY }) => {
        const size = shownCanvasSize();
        if (!size || imageX === null || imageY === null) return;
        const share = (value: number) => Math.min(Math.max(value, 0), 1);
        const point: [number, number] = [share(imageX / size.width), share(imageY / size.height)];
        setPoint(name, point);
        onPick(point);
      }),
    /** Takes the gizmo `name` away. */
    remove: (name: string) => {
      for (const part of [`${name}:edge`, `${name}:halo`, `${name}:dot`, name]) editorGizmos.value?.remove(part);
    },
  },
  gestures: {
    /**
     * Says which two-finger gestures the plugin takes (`events.pinches`); the rest zoom, turn and move the preview. `claim` is asked as the
     * second finger lands, with the middle between the fingers as in `events.taps`, and `imageScale`, how many photo pixels are one preview
     * pixel there. Returns what cancels the claim. Without a claim a plugin that listens takes every pinch.
     */
    claimPinch: (claim: (point: { x: number; y: number; imageX: number | null; imageY: number | null; imageScale: number }) => boolean) => {
      const test = (pageX: number, pageY: number) => {
        const point = { x: pageX - previewOrigin.value.x, y: pageY - previewOrigin.value.y };
        const at = onPhoto(point);
        const next = onPhoto({ x: point.x + 1, y: point.y });
        const imageScale = at.imageX === null || next.imageX === null || at.imageY === null || next.imageY === null ? 1 : Math.hypot(next.imageX - at.imageX, next.imageY - at.imageY);
        return claim({ ...point, ...at, imageScale });
      };
      pluginPinchClaim.next(test);
      return {
        unsubscribe: () => {
          if (pluginPinchClaim.value === test) pluginPinchClaim.next(null);
        },
      };
    },
  },
  /** Fonts downloaded on the Fonts screen, and text set in them. The editor writes text through the same functions. */
  fonts: {
    /** The names of the families on this device. */
    installed: () => installedFamilies().map(each => each.family),
    /** The text as a new transparent image sized around the letters, which the caller releases, or null when there is nothing to write. */
    renderText,
    /** Writes `spec.text` on `layer`, centered across `box` and lined up inside it as `vertical` says. The layer is hidden when there is nothing to write. */
    placeText: (
      layer: LayerLike,
      box: { x: number; y: number; width: number; height: number },
      spec: Parameters<typeof renderText>[0],
      vertical: 'top' | 'middle' | 'bottom' = 'middle',
    ) => {
      const image = renderText(spec);
      if (!image) {
        layer.setVisible(false);
        return;
      }
      const [width, height] = [image.width(), image.height()];
      const free = box.height - height;
      layer.setImage(image);
      layer.setPosition(Math.round(box.x + (box.width - width) / 2), Math.round(box.y + (vertical === 'top' ? 0 : vertical === 'bottom' ? free : free / 2)));
      layer.setVisible(true);
      release(image);
    },
  },
  editor: {
    currentImage: () => currentImage.value,
    hasHistory: () => historyReady.value,
    flush: () => flushFullReplay(),
    area: (key: string) => areaRings(areas.value, key),
    setArea: (key: string, rings: number[][][]) => setArea(key, rings),
    setBusy: (value: boolean) => busy.next(value),
    replaceImage: (image: AbraImageLike, result: ImageLike) =>
      image.replaceWith(result as Parameters<AbraImageLike['replaceWith']>[0]),
    subjects: () => framedPeople() ?? [],
    /** Gives `change` a copy of the current image and, when it returns true, makes the copy the new current image. The copy is released either way. */
    edit: (change: (photo: ImageLike) => boolean | Promise<boolean>): boolean | Promise<boolean> => {
      if (!historyReady.value) return false;
      flushFullReplay();
      const image = currentImage.value;
      if (!image) return false;
      const photo = image.clone();
      let freed = false;
      const free = () => {
        if (freed) return;
        freed = true;
        release(photo);
      };
      const commit = (changed: boolean) => {
        if (changed) image.replaceWith(photo as Parameters<AbraImageLike['replaceWith']>[0]);
        return changed;
      };
      try {
        const changed = change(photo);
        if (changed && typeof (changed as Promise<boolean>).then === 'function') {
          return Promise.resolve(changed).then(commit).finally(free);
        }
        const result = commit(changed as boolean);
        free();
        return result;
      } catch (error) {
        free();
        throw error;
      }
    },
  },
  /** Read-only streams of what changes in the editor. `plugin-host.ts` hands each plugin a copy that cancels its subscriptions when it closes. */
  events: {
    areas: areas$,
    history: timeline$,
    historyReady: historyReady$,
    adjustments: adjustments$,
    people: people$,
    aiProgress: aiProgress$,
    taps: previewPressed$.pipe(map(tap => ({ x: tap.x, y: tap.y, ...onPhoto(tap) }))),
    pinches: new Observable<PluginPinch>(subscriber => {
      pluginPinchListeners.next(pluginPinchListeners.value + 1);
      const subscription = pluginPinch$.subscribe(subscriber);
      return () => {
        subscription.unsubscribe();
        pluginPinchListeners.next(pluginPinchListeners.value - 1);
      };
    }).pipe(
      map(pinch => {
        // The photo-space move is the difference between where the middle is now and where it was, so a turned or zoomed preview is allowed for.
        const now = onPhoto(pinch);
        const before = onPhoto({ x: pinch.x - pinch.dx, y: pinch.y - pinch.dy });
        const [imageDx, imageDy] =
          now.imageX === null || now.imageY === null || before.imageX === null || before.imageY === null
            ? [null, null]
            : [now.imageX - before.imageX, now.imageY - before.imageY];
        return { ...pinch, ...now, imageDx, imageDy };
      }),
    ),
    touches: new Observable<PluginTouch>(subscriber => {
      pluginTouchListeners.next(pluginTouchListeners.value + 1);
      const subscription = pluginTouch$.subscribe(subscriber);
      return () => {
        subscription.unsubscribe();
        pluginTouchListeners.next(pluginTouchListeners.value - 1);
      };
    }).pipe(map(touch => ({ type: touch.type, x: touch.x, y: touch.y, ...onPhoto(touch) }))),
  },
  ai: {
    models: () => aiModelList.value,
    selectedModel: (key: string) => toolAiModelSelections.value[key] ?? null,
    setSelectedModel: (key: string, role: string) => setToolAiModel(key, role),
    missing: (roles: readonly string[]) => missingModels(roles),
    requestModels: (title: string, roles: readonly string[], onReady: () => void) =>
      aiModelsRequested.next({ title, roles, onReady }),
    /** The model of `kind` the user picked for `key` (else the first enabled one), once it is on the device; null when there is none, or after asking for it (`retry` runs when it arrives). */
    ensureModel: (key: string, kind: string, title: string, retry: () => void) => {
      const models = aiModelList.value.filter(model => model.enabled && model.kinds.includes(kind));
      const selected = toolAiModelSelections.value[key];
      const model = models.find(item => item.id === selected) ?? models[0];
      if (!model) {
        setToolMessage(key, `No ${title.toLowerCase()} model is listed. Check your connection and try again.`);
        return null;
      }
      if (missingModels([model.id]).length > 0) {
        aiModelsRequested.next({ title, roles: [model.id], onReady: retry });
        return null;
      }
      return model;
    },
    /**
     * Runs `work` as one AI job: the editor is busy, the pending notice is up and `progress` is reported until it ends. A failure becomes
     * the message under `key`, led by `failure`, and gives false.
     */
    job: async (options: { key: string; progress: string; notice: AiPendingNoticeText; failure: string }, work: () => Promise<void>): Promise<boolean> => {
      busy.next(true);
      setToolMessage(options.key, null);
      setAiPendingNotice(options.notice);
      try {
        await app.ai.withProgress(options.progress, work);
        return true;
      } catch (error) {
        setToolMessage(options.key, `${options.failure}: ${error instanceof Error ? error.message : String(error)}`);
        return false;
      } finally {
        setAiPendingNotice(null);
        busy.next(false);
      }
    },
    prepareModel: (owner: string, role: string) =>
      registerAiModel({ owner, role, lifetime: aiModelLifetimes.multiShot }),
    releaseModel: (owner: string, role?: string) => releaseAiModel(owner, role),
    withModel: <T>(owner: string, role: string, work: (model: AiModelType) => Promise<T>) =>
      runWithAiModel({ owner, role, lifetime: aiModelLifetimes.multiShot }, model => work(model as AiModelType)),
    /** Runs a model file with a plugin-supplied manifest, freeing the native model when the work ends. */
    withManifest: async <T>(role: string, manifest: string, work: (model: AiModelType) => Promise<T>): Promise<T> => {
      const model = await AiModel.loadWithManifest(aiModelPath(role), manifest);
      try {
        return await work(model as AiModelType);
      } finally {
        release(model);
      }
    },
    /**
     * Runs a class-based segmentation model (`semantic-mask` decoder) over `photo` and gives the mask of `maskClasses`, or undefined when
     * nothing was found. `classes` names every class the model outputs, in order. The caller releases the mask.
     */
    segment: async (
      role: string,
      photo: ImageLike,
      spec: { name: string; inputSize: number; classes: string[]; maskClasses: number[] },
    ): Promise<MaskLike | undefined> => {
      const manifest = `name: ${spec.name}
description: ${spec.name}
decoder: semantic-mask
params:
  input_size: ${spec.inputSize}
  input_layout: nhwc
  input_fit: stretch
  output_layout: nhwc
  class_count: ${spec.classes.length}
  mask_classes: [${spec.maskClasses.join(', ')}]
classes: [${spec.classes.join(', ')}]
`;
      const output = await app.ai.withManifest(role, manifest, model => runAiModelAsync(model, photo as Parameters<typeof runAiModelAsync>[1]));
      try {
        const mask = output.mask();
        if (mask.bounds(0)) return mask;
        release(mask);
        return undefined;
      } finally {
        release(output);
      }
    },
    /** Runs an image model directly and returns its typed model result. */
    run: (model: AiModelType, image: ImageLike) =>
      runAiModelAsync(model, image as Parameters<typeof runAiModelAsync>[1]),
    runGuided: (model: AiModelType, image: ImageLike, mask: MaskLike) =>
      runAiModelGuidedAsync(
        model,
        image as Parameters<typeof runAiModelGuidedAsync>[1],
        mask as Parameters<typeof runAiModelGuidedAsync>[2],
      ),
    withProgress: async <T>(key: string, work: () => Promise<T>): Promise<T> => {
      aiProgress.next({ ...aiProgress.value, [key]: null });
      let failed = false;
      try {
        return await work();
      } catch (error) {
        failed = true;
        throw error;
      } finally {
        const { [key]: _finished, ...others } = aiProgress.value;
        aiProgress.next(failed ? others : { ...aiProgress.value, [key]: 1 });
      }
    },
  },
  history: {
    commitRetouch: (change: { type: string; [key: string]: unknown }) =>
      commitRetouch(change as Parameters<typeof commitRetouch>[0]),
    save: () => saveOpenProject(),
  },
  tools: { setMessage: (key: string, message: string | null) => setToolMessage(key, message) },
};

/**
 * What a control script of a plugin exports as its default: a function that returns the control. The plugin's `plugin.yml` says where
 * the control goes and what it is called, so the control does not.
 */
