import type { AiModel } from '@abra/core';
import type { BlendMode } from '@abra/core/blend';
import type { CanvasLike } from '@abra/core/canvas';
import type { ColorLike } from '@abra/core/color';
import type { Rect } from '@abra/core/geometry';
import type { Image, ImageLike } from '@abra/core/image';
import type { MaskLike } from '@abra/core/mask';

/** A stream of values from the editor that a plugin can listen to. */
export type PluginStream<T> = {
  subscribe(listener: (value: T) => void): { unsubscribe(): void };
};
/** Set temporary text for the app's AI pending notice. Set to null to restore its generic text. */
export declare function setAiPendingNotice(value: AiPendingNoticeText | null): void;
/** Prefix a plugin-owned control key with the current plugin ID. */
export declare function toolKey(id: string): string;
/** A local file copied into the plugin's accessible storage by the inspector picker. */
export type PickedFile = {
  /** URI the plugin can use to read the copied file. */
  uri: string;
  /** Original file name, when available. */
  name?: string;
  /** File size in bytes, when available. */
  size?: number | null;
  /** MIME type, when available. */
  mimeType?: string | null;
  /** URI of the original file copy, when provided by the platform. */
  fileCopyUri?: string | null;
};
/** Adds a native image picker to the plugin editor inspector. The callback receives the full list of copied local files each time images are added or removed. */
export declare const inspector: {
  /** Add a file picker. Set `source` to choose where images come from (`'file-system'` by default, `'projects'` for the app's saved projects, whose edited image is picked, or `'both'`), `multiple` to allow multiple files and `thumbs` to show image thumbnails. */
  filePicker(label: string, options: { source?: 'file-system' | 'projects' | 'both'; multiple?: boolean; thumbs?: boolean; onPick(files: PickedFile[]): void }): void;
  /** Adds a heading to the plugin editor inspector, like an <h1> to <h6>. `level` defaults to 1. `back` shows a Back button with the heading and calls it when it is tapped: for the top of a sub-inspector, to return to the main one. */
  header(text: string, options?: { level?: 1 | 2 | 3 | 4 | 5 | 6; back?(): void }): void;
  /** Starts a foldable group in the plugin editor inspector. Every control added after it, up to the next `group`, is inside it, and tapping the group's title folds or unfolds them. Controls added before the first `group` are always showing. `open` is whether it starts unfolded, and defaults to false; once the person has tapped a group, their choice is kept while the editor is open. Pass `enabled` to put a checkbox in the title, ticked when it is true, for a group that is switched on and off as a whole (like a module of a particle system): `onToggle` gets the new value when it is tapped and the editor runs the plugin again after it. The title of a group that is off is dimmed, and its controls stay so they can still be set. */
  group(label: string, options?: { open?: boolean; enabled?: boolean; onToggle?(enabled: boolean): void }): void;
  /** Runs the plugin's `inspectorGUI` and `previewCanvas` again, for a change that did not come from one of its controls (a tap on the preview, say). The inspector can then show another set of controls, such as one for what was tapped. */
  refresh(): void;
  /** Says the canvas `previewCanvas` returned was changed in place (a layer moved, turned or scaled), so the preview draws it again at once. A slider says this itself; a change that comes from a gesture says it here. */
  redraw(): void;
  /** Opens the inspector over the preview, on a screen too narrow to show it beside the preview, for when the plugin has something to show there (the settings of what was tapped). It does nothing where the inspector is always showing. */
  open(): void;
  /** Adds two number fields side by side, like a size or a point. `value` defaults to `[0, 0]` and `labels` to `['Width', 'Height']`. `min` and `max` limit both numbers. `onChange` gets both numbers each time one is changed to a number. */
  size(
    label: string,
    options?: {
      value?: [number, number];
      labels?: [string, string];
      reset?: boolean;
      min?: number;
      max?: number;
      onChange?(value: [number, number]): void;
    },
  ): void;
  /** Adds a bar of buttons to pick one of `options`, across the whole width. `value` defaults to the first. With `momentary` the bar is a row of buttons that act when tapped and never stay selected. The editor runs the plugin again after `onChange`. */
  select(label: string, options: { options: string[]; value?: string; momentary?: boolean; onChange?(value: string): void }): void;
  /** Adds a box for a line of text. `value` defaults to nothing and `placeholder` is shown while it is empty. `onChange` gets the text a moment after typing stops, and the editor runs the plugin again after it. */
  text(label: string, options?: { value?: string; placeholder?: string; onChange?(value: string): void }): void;
  /** Adds a button showing the chosen font family (`value`, or "Choose a font"). Tapping it opens the Fonts overlay, where the user browses the fonts on the device and downloads more. `onPick` gets the family, which works with `app.fonts.renderText`; the editor runs the plugin again after it. */
  fontPicker(label: string, options?: { value?: string; onPick?(family: string): void }): void;
  /** Adds a grid of thumbnails of `files` to choose from, `rows` high (default 2) with the rest reached by scrolling sideways, or every one of them when `rows` is `'all'`. `onPick` gets the place in `files` of the one tapped, and the editor runs the plugin again after it. */
  pictures(label: string, options: { files: readonly PickedFile[]; rows?: number | 'all'; onPick?(index: number): void }): void;
  /** Adds an on/off switch. `value` defaults to false. The editor runs the plugin again after `onChange`, which gets whether it is on. */
  toggle(label: string, options?: { value?: boolean; onChange?(value: boolean): void }): void;
  /**
   * Adds a color. It shows a swatch of the color and its web color, and opens the app's color picker when tapped: a square for how colorful and
   * how bright, a bar for which color, named colors for the common choices, and boxes to type a color. `value` is `[red, green, blue]` (each 0 to
   * 255, white by default), with the strength as a fourth number when `alpha` is true. `title` is the words over the picker (the control's name by
   * default) and `swatches` the named colors it shows. `onChange` gets the color, in the same form, when the person presses OK, and the editor runs the
   * plugin again after it. `onPreview` gets the color as the person drags in the picker, before OK, so a plugin can show it at once; if the picker is cancelled it
   * is called once more with the color the control had. The editor does not run the plugin again for it: change what is showing, as a slider's `onChange` does.
   */
  color(
    label: string,
    options?: {
      value?: number[];
      alpha?: boolean;
      title?: string;
      swatches?: { label: string; rgb: [number, number, number] }[];
      onChange?(value: number[]): void;
      onPreview?(value: number[]): void;
    },
  ): void;
  /**
   * Adds a gradient: a bar showing it, which opens the gradient editor when it is tapped. `value` is `{ colors, alphas }`: `colors` are `{ at, color: [red, green, blue] }` and `alphas` are `{ at, alpha }`, with `at` from 0 (the start) to 1 (the end) and `alpha` from 0 (not there) to 255 (solid). The strength has stops of its own, so it need not change where the color does, as in Unity's gradient editor. Each list needs a stop (one that has none gets a white, or a solid, one), and the default is solid white. `title` is the words over the editor (the control's name by default) and `swatches` the named colors the color picker of a stop shows. `onChange` gets the gradient, each list in order along it, when the person presses OK, and the editor runs the plugin again after it. `onPreview` gets it as it is edited, before OK, and once more with the old gradient if the editor is cancelled; the editor does not run the plugin again for it.
   */
  gradient(
    label: string,
    options?: {
      value?: { colors: { at: number; color: number[] }[]; alphas: { at: number; alpha: number }[] };
      title?: string;
      swatches?: { label: string; rgb: [number, number, number] }[];
      onChange?(value: { colors: { at: number; color: [number, number, number] }[]; alphas: { at: number; alpha: number }[] }): void;
      onPreview?(value: { colors: { at: number; color: [number, number, number] }[]; alphas: { at: number; alpha: number }[] }): void;
    },
  ): void;
  /**
   * Adds a curve: a small drawing of it across the row, which opens the curve editor when it is tapped. `value` is the points, each `[x, y]`, with `x` from 0 (the start) to 1 (the end) and `y` from `min` to `max` (0 and 1 by default, and the range of the editor). The curve is a smooth line through the points, flat before the first and after the last, and the editor draws it with the library's own curve, so it is the line the plugin makes. A curve needs a point; with none it is flat at `max`. `presets` are named shapes the editor offers to start from, each `{ label, value }`. `title` is the words over the editor (the control's name by default). `onChange` gets the points, in order along the curve, when the person presses OK, and the editor runs the plugin again after it. `onPreview` gets them as the curve is edited, before OK, and once more with the old points if the editor is cancelled; the editor does not run the plugin again for it.
   */
  curve(
    label: string,
    options?: {
      value?: [number, number][];
      min?: number;
      max?: number;
      title?: string;
      presets?: { label: string; value: [number, number][] }[];
      onChange?(value: [number, number][]): void;
      onPreview?(value: [number, number][]): void;
    },
  ): void;
  /** Adds one number field.`value` defaults to `min` (or 0), `step` to 1, and `reset` to true. `min` and `max` limit the number. The editor runs the plugin again after `onChange`. */
  number(
    label: string,
    options?: {
      value?: number;
      min?: number;
      max?: number;
      step?: number;
      reset?: boolean;
      onChange?(value: number): void;
    },
  ): void;
  /** Adds a slider to the plugin editor inspector. `min` defaults to 0, `step` to 1 and `range` to false (one thumb). */
  slider(
    label: string,
    options: {
      min?: number;
      max: number;
      step?: number;
      /** The name on a line of its own with the slider under it. Defaults to false, and to true when `range` is true. */ multiline?: boolean;
      /** A reset button beside the name, which puts the slider back to where it was when the editor opened. Defaults to true. */ reset?: boolean;
      /** Runs the plugin again once the slider stops moving, for a value that changes how the picture is made. Defaults to false. */ rebuild?: boolean;
    } & (
      | { range?: false; value?: number; onChange?(value: number): void }
      | { range: true; value?: [number, number]; onChange?(value: [number, number]): void }
    ),
  ): void;
};
/** A random number generator that makes the same numbers every time it is given the same seed. */
export declare function seededRandom(s: string | number): import('rand-seed').default;
/** A seeded random number generator instance, created by `seededRandom`. */
export type Rand = import('rand-seed').default;
/** A whole number from `min` to `max`, either of which it can be, drawn from `rand`. */
export declare function randomBetween(rand: Rand, min: number, max: number): number;
/** The numbers 0 up to `count` in an order drawn from `rand`. */
export declare function shuffled(count: number, rand: Rand): number[];
/** The circle round each ring of an area (rings are `[x, y]` fractions of the photo), in pixels of a photo `width` by `height`: the middle of the ring's box and half its diagonal. */
export declare function areaSpots(rings: number[][][], width: number, height: number): { x: number; y: number; radius: number }[];

export type AiPendingNoticeText = { text: string; subText?: string; detail?: string };
/** Summary of an AI model available to the app. */
export type AppModel = {
  id: string;
  title: string;
  description: string;
  enabled: boolean;
  kinds: string[];
  downloaded: boolean;
};
/** Generic app services available to plugin controls. */
export declare const app: {
  /** What a plugin keeps with its project, for state no control holds (which picture a cell was swapped for, say). Plain data only; saved with the project a moment after it changes and given back, before the plugin runs, when the project opens. */
  saved: {
    /** What was kept under `name`, or `undefined` when nothing was. */
    get(name: string): unknown;
    /** Keeps `value` under `name`. */
    set(name: string, value: unknown): void;
  };
  /** The files the plugin ships (its `assets` in plugin.yml, such as sprites). */
  assets: {
    /** The path on the device of an asset, given its path relative to the plugin's folder (`'sprites/spark.png'`). Throws if the plugin shipped no such file. */
    path(relative: string): string;
  };
  /**
   * What the plugin keeps for itself across projects, as named text (write JSON): a particle maker's systems, so any project can use them
   * later. Each plugin has its own saves and cannot reach another's. They are kept beside the plugin and not in it, so removing the plugin
   * does not remove them. A name is letters, digits, spaces, `-`, `_` and `.` (at most 100, not starting with a dot) and a save is at most
   * 5 MB. These throw when a name is not allowed, a save is too big, or the device cannot write it.
   */
  saves: {
    /** The names of the plugin's saves, in order. */
    list(): string[];
    /** The text of a save, or `undefined` when there is none with that name. */
    read(name: string): string | undefined;
    /** Keeps `text` under `name`, replacing what was there. */
    write(name: string, text: string): void;
    /** Removes a save. It does nothing when there is none with that name. */
    remove(name: string): void;
  };
  /** Which gestures on the preview the plugin takes. */
  gestures: {
    /**
     * Says which two-finger gestures (`events.pinches`) the plugin takes; the rest zoom, turn and move the preview. `claim` is asked once, as
     * the second finger lands, with the middle between the fingers (`x`, `y`, `imageX`, `imageY` as in `events.taps`) and `imageScale`, how
     * many photo pixels are one preview pixel there. Return true to take the pinch. Without a claim, a plugin that listens takes every pinch.
     * Returns what cancels the claim, which is also cancelled when the plugin closes.
     */
    claimPinch(claim: (point: { x: number; y: number; imageX: number | null; imageY: number | null; imageScale: number }) => boolean): { unsubscribe(): void };
  };
  /** The app's color picker, which `inspector.color` opens too. */
  colors: {
    /** Opens the color picker. Answers with the color when the person presses OK, as `[red, green, blue]` (each 0 to 255) and the strength as a fourth number when `alpha` is true, or `null` when they cancel. `value` is where it starts (white by default) and `swatches` the named colors it shows. */
    pick(options?: {
      title?: string;
      value?: number[];
      alpha?: boolean;
      swatches?: { label: string; rgb: [number, number, number] }[];
      /** Called with the color as the person drags, before OK, and once more with the starting color if they cancel. */
      onPreview?(value: number[]): void;
    }): Promise<number[] | null>;
  };
  /** Gizmos: shapes the app draws over the preview. They follow the picture as it is zoomed, moved and turned, and go away when the workspace closes. Positions are in pixels of the canvas `previewCanvas` returned. */
  gizmos: {
    /** Sets the color of the gizmos drawn from now on, until it is set again. Gizmos already drawn keep theirs. `null` goes back to the default, white. */
    color(color: ColorLike | null): void;
    /** Sets how wide the lines of the gizmos drawn from now on are, in screen pixels. The default is 3. */
    size(pixels: number): void;
    /** Draws a frame round four corners (each `[x, y]`: top left, top right, bottom right, bottom left) as the gizmo `name`, in the color and width set. A soft dark line is drawn under it so it shows on any picture. Setting a name again moves it. `canvas.quadInParent()` gives the corners of a canvas inside another, and `canvas.childAt(x, y)` finds the canvas on top at a point. */
    setBox(name: string, corners: number[][]): void;
    /**
     * Draws a dot as the gizmo `name` at `point`, as shares of the canvas `previewCanvas` returned: `[0, 0]` is its top left and `[1, 1]` its bottom
     * right. The dot has a dark halo, a white ring and the gizmo color in the middle, stays the same size on the screen at any zoom, and
     * follows the picture. Setting a name again moves it. It is for any point the person chooses: the source of an effect, the end of a gradient.
     * `previewCanvas` runs before the app shows the canvas it returns, so a dot placed from it names that canvas: `setPoint(name, point, canvas)`.
     * Without one, the canvas on show is used.
     */
    setPoint(name: string, point: [number, number], canvas?: CanvasLike): void;
    /**
     * Lets the person tap the preview to choose a point. Each tap on the canvas calls `onPick` with the point as shares of the canvas's width and
     * height (`[0, 0]` the top left, `[1, 1]` the bottom right) and puts the dot gizmo `name` there; a tap off the canvas does nothing. Run
     * `inspector.refresh()` in `onPick` when the choice changes what the plugin makes. Returns what stops it, which is also stopped when the
     * plugin closes.
     */
    pickPoint(name: string, onPick: (point: [number, number]) => void): { unsubscribe(): void };
    /** Takes the gizmo `name` away. */
    remove(name: string): void;
  };
  /** Fonts downloaded on the Fonts screen, and text set in them. */
  fonts: {
    /** The names of the families on this device. */
    installed(): string[];
    /** Writes `text` in a family of `installed` as a new transparent image sized around the letters, which the caller releases. `size` is the size of the letters in pixels, `width` wraps the text to that many pixels and `align` lines it up (left by default). Null when there is nothing to write: no text, or a family that is not on this device. */
    renderText(spec: { text: string; family: string; size: number; color?: import('@abra/core/color').ColorLike; width?: number; align?: 'left' | 'center' | 'right' }): ImageLike | null;
    /** Writes `spec.text` (as `renderText`) on `layer`, centered across `box` and lined up inside it as `vertical` says (middle by default). The layer is hidden when there is nothing to write. */
    placeText(
      layer: import('@abra/core/canvas').LayerLike,
      box: Rect,
      spec: { text: string; family: string; size: number; color?: import('@abra/core/color').ColorLike; width?: number; align?: 'left' | 'center' | 'right' },
      vertical?: 'top' | 'middle' | 'bottom',
    ): void;
  };
  /** Access the current editor image, selection areas, and editor state. */
  editor: {
    /** Current editor image, or null when no image is open. */
    currentImage(): ImageLike | null;
    /** Whether an undo or redo operation is available. */
    hasHistory(): boolean;
    /** Flush pending editor updates. */
    flush(): void;
    /** Read saved polygon rings for an area key. */
    area(key: string): number[][][];
    /** Save polygon rings for an area key. */
    setArea(key: string, rings: number[][][]): void;
    /** Show or clear the editor's busy state. */
    setBusy(value: boolean): void;
    /** Replace the current image with the result while retaining the original for history. */
    replaceImage(image: ImageLike, result: ImageLike): void;
    /** Detected subjects with their bounds and masks when analysis is available. */
    subjects(): { id: number; x: number; y: number; width: number; height: number; mask: MaskLike }[];
    /** Gives `change` a copy of the current image to edit and, when it returns true, makes the copy the editor's image. The copy is released either way. False when no project is open or nothing changed. */
    edit(change: (photo: ImageLike) => boolean): boolean;
    edit(change: (photo: ImageLike) => Promise<boolean>): Promise<boolean>;
  };
  /**
   * Read-only streams of what changes in the editor. Call `subscribe` on one to listen; the app cancels every subscription when the plugin
   * closes, so a plugin never has to unsubscribe. The returned subscription can still be cancelled earlier with `unsubscribe()`. A stream that
   * holds a current value (areas, history, adjustments, people, aiProgress, historyReady) calls the listener once right away with it.
   */
  events: {
    /** Every area the user has drawn, as polygon points per area key. */
    areas: PluginStream<{ key: string; points: [number, number][] }[]>;
    /** The edit history: every step, and how many are applied. */
    history: PluginStream<{ entries: readonly unknown[]; cursor: number }>;
    /** Whether the edit history is ready. It is false until the editor session has opened. */
    historyReady: PluginStream<boolean>;
    /** The value of every adjustment control, by control key. */
    adjustments: PluginStream<Record<string, unknown>>;
    /** People found in the image, or null before analysis has run. */
    people: PluginStream<{ id: number; x: number; y: number; width: number; height: number }[] | null>;
    /** Progress of each running AI task from 0 to 1, or null while its length is unknown. */
    aiProgress: PluginStream<Record<string, number | null>>;
    /**
     * A tap on the preview. `x` and `y` are in the preview's pixels from its top-left; `imageX` and `imageY` are the same spot in the photo's
     * pixels, or null when the tap was off the photo. Does not fire while a plugin listens to `touches`.
     */
    taps: PluginStream<{ x: number; y: number; imageX: number | null; imageY: number | null }>;
    /**
     * One finger on the preview, from `down` through `move` to `up` (or `cancel` if a second finger or the app took it). Coordinates are as in
     * `taps`. While a plugin listens, a single finger belongs to the plugin and does not pan the photo; two fingers still pinch and pan. The
     * touch layer is removed again when the plugin closes.
     */
    touches: PluginStream<{ type: 'down' | 'move' | 'up' | 'cancel'; x: number; y: number; imageX: number | null; imageY: number | null }>;
    /**
     * Two fingers on the preview, from `start` through `change` to `end`. `x` and `y` are the middle between the fingers, as in `taps` (with
     * `imageX` and `imageY` the same spot in the photo's pixels, or null off the photo). `scale` is how far apart the fingers are compared with
     * at `start` (1 at `start`), `rotation` is how far the line between them has turned since `start`, in degrees, and `dx` and `dy` is how far
     * their middle has moved in the preview's pixels (`imageDx` and `imageDy` in the photo's, or null off the photo). While a plugin listens,
     * two fingers belong to the plugin and do not zoom, turn or move the preview, unless `gestures.claimPinch` says otherwise; one finger still
     * pans it. Removed again when the plugin closes.
     */
    pinches: PluginStream<{
      type: 'start' | 'change' | 'end';
      x: number;
      y: number;
      imageX: number | null;
      imageY: number | null;
      scale: number;
      rotation: number;
      dx: number;
      dy: number;
      imageDx: number | null;
      imageDy: number | null;
    }>;
  };
  /** Select, prepare, run, and release AI models. */
  ai: {
    /** List models known to the app. */
    models(): readonly AppModel[];
    /** Return the selected model ID for a plugin-defined key, or null. */
    selectedModel(key: string): string | null;
    /** Select a model for a plugin-defined key and role. */
    setSelectedModel(key: string, role: string): void;
    /** Return model roles that are not currently available. */
    missing(roles: readonly string[]): string[];
    /** Ask the app to request missing model roles and invoke `onReady` when ready. */
    requestModels(title: string, roles: readonly string[], onReady: () => void): void;
    /** The enabled model of `kind` the user chose for `key`, else the first one. Null when there is none (a message is shown under `key`) or it is not on the device yet (it is asked for, and `retry` runs once it arrives). `title` names the feature in both. */
    ensureModel(key: string, kind: string, title: string, retry: () => void): AppModel | null;
    /** Runs `work` as one AI job: the editor is busy, `notice` is shown and `progress` is reported until it ends. A failure is shown as a message under `key`, led by `failure`. Resolves to whether `work` finished. */
    job(options: { key: string; progress: string; notice: AiPendingNoticeText; failure: string }, work: () => Promise<void>): Promise<boolean>;
    /** Prepare a model for use by the named owner and role. */
    prepareModel(owner: string, role: string): Promise<void>;
    /** Release models prepared by an owner, optionally limited to one role. */
    releaseModel(owner: string, role?: string): void;
    /** Run asynchronous work with a prepared model. */
    withModel<T>(owner: string, role: string, work: (model: AiModel) => Promise<T>): Promise<T>;
    /** Run a model file with a caller-supplied manifest and release the model afterward. */
    withManifest<T>(role: string, manifest: string, work: (model: AiModel) => Promise<T>): Promise<T>;
    /** Runs a class-based segmentation model (`semantic-mask` decoder) over `photo` and gives the mask of `maskClasses`, or undefined when nothing was found. `classes` names every class the model outputs, in order. The caller releases the mask. */
    segment(role: string, photo: ImageLike, spec: { name: string; inputSize: number; classes: string[]; maskClasses: number[] }): Promise<MaskLike | undefined>;
    /** Run an image model and get its result (for example, a segmentation mask). */
    run(model: AiModel, image: ImageLike): Promise<{ mask(): MaskLike }>;
    runGuided(model: AiModel, image: ImageLike, mask: MaskLike): Promise<{ image(): ImageLike }>;
    /** Run asynchronous work while reporting progress under a plugin-owned key. */
    withProgress<T>(key: string, work: () => Promise<T>): Promise<T>;
  };
  /** Add completed edits to history and save editor changes. */
  history: {
    /** Commit a plugin-defined retouch change to editor history. */
    commitRetouch(change: { type: string; [key: string]: unknown }): void;
    /** Persist pending editor changes. */
    save(): Promise<void>;
  };
  /** Show temporary messages associated with plugin tools. */
  tools: { setMessage(key: string, message: string | null): void };
};
/** Detect facial regions in an image when analysis is available. */
export declare function faceMasks(photo: ImageLike): Promise<Record<string, MaskLike> | undefined>;
/** Rasterize polygon rings into a mask of the given dimensions. `keep` can retain the mask for later use. */
export declare function areaMask(rings: number[][][], width: number, height: number, keep: Keep): MaskLike;
/** Test whether a face or region identified by `box` belongs to the supplied person bounds. */
export declare function belongsTo(
  box: string,
  photo: { width(): number; height(): number },
  person: { x: number; y: number; width: number; height: number } | undefined,
  others?: number[],
): boolean;
/** Find faces with their parsed regions, crop positions and landmark results for general facial effects. */
export declare function findFaces(
  photo: ImageLike,
  keep: Keep,
): AsyncGenerator<{
  body: Rect;
  labels: { maskNamed(names: string[]): MaskLike };
  zoom: Rect;
  face: Rect;
  faceImage: ImageLike;
  landmarks: { landmarks: { point: { x: number; y: number } }[] };
}>;
export declare function derivedFor(
  owner: Preloaded,
  group: string,
  variant: string,
  make: () => DataLike | undefined,
): DataLike | undefined;
/** Retain values for the lifetime of the current tool execution. */
export type Keep = {
  <V>(value: V): V;
  <V extends unknown[]>(...values: V): V;
};
/** Run work with a `keep` function for retaining returned values. */
export declare function owning<T>(work: (keep: Keep) => T): T;
/** Release retained images, masks, or other toolkit values after use. */
export declare function release(...values: unknown[]): void;
/** Paint masked regions onto a painting sheet with the given color. */
export declare function paintSheet(sheet: Painting, regions: MaskLike[], color: Color): void;
/** Attach per-color values generated by `values` to each named swatch. */
export declare function swatchesOf(
  colors: { label: string; color: string }[],
  values: (hue: number, saturation: number, brightness: number, label: string) => (number | undefined)[],
): { label: string; color: string; values: (number | undefined)[] }[];

/** Context passed to plugin callbacks while a tool runs. */
export type ToolContext = {
  /** Current editor-to-image scale factor. */
  scale: number;
  /** Data prepared by the control's preload function. */
  preloaded?: Preloaded;
  /** Saved polygon areas corresponding to the control's `areaKeys`. */
  areas?: (number[][][] | undefined)[];
  /** Bounds of the currently selected person, when present. */
  person?: { x: number; y: number; width: number; height: number; id?: number };
  /** Lazily obtain the current skin mask, when available. */
  skin?: () => MaskLike | undefined;
  /** IDs of other detected subjects. */
  others?: number[];
  /** Current numeric control values. */
  values?: number[];
};
/** Image or mask data a plugin can preload or cache. */
export type DataLike = ImageLike | MaskLike;
/** Values returned from a control's preload hook. */
export type Preloaded = DataLike[] | Record<string, DataLike>;
/** Synchronous or asynchronous result accepted from a preload hook. */
export type Preloading =
  | void
  | DataLike
  | DataLike[]
  | Record<string, DataLike>
  | Promise<void | DataLike | DataLike[] | Record<string, DataLike>>;
export type PerSlider<T> = T | (T | undefined)[];
/** A single numeric setting or a low/high pair for a range control. */
export type RangeSetting = number | [number, number];
export type AdjustmentValue = number | number[];
export type PillFormat = 'signed' | 'percent' | 'plain';
/** Drawing surface passed to layer paint callbacks. */
export type Painting = {
  photo: ImageLike;
  width(): number;
  height(): number;
  painted?: { image: ImageLike; x: number; y: number };
};

/** Configuration for a slider control, including its tool, preload, and optional painted layer. */
export type SliderControl<T = unknown> = {
  kind: 'slider';
  display?: 'all' | 'tabulated';
  pillFormat?: PerSlider<PillFormat>;
  valueText?: PerSlider<(value: number) => string | undefined>;
  key: string;
  label: string;
  person?: boolean;
  face?: boolean;
  needsModels?: { title: string; roles: readonly string[] };
  /** Stable name for project-persisted analysis masks produced by this preload. */
  analysisGroup?: string;
  sliderLabel?: PerSlider<string>;
  min?: PerSlider<RangeSetting>;
  max?: PerSlider<RangeSetting>;
  step?: PerSlider<RangeSetting>;
  defaultValue?: PerSlider<RangeSetting>;
  areaKeys?: string[];
  applyOnSelect?: boolean;
  apply?: (image: Image, value: number, context: ToolContext) => void;
  picker?: PerSlider<'skin-tan' | 'hue' | 'shade' | 'glow' | 'infrared' | 'neon' | 'swatches'>;
  swatches?: { label: string; color: string; values: (number | undefined)[] }[];
  preload?: (photo: ImageLike) => Preloading | Promise<Preloading>;
  tool?(photo: ImageLike, value: T[], context: ToolContext): ImageLike | void;
  joint?: {
    key: string;
    sliders: SliderControl[];
    tool(photo: ImageLike, values: number[][], context: ToolContext): ImageLike | void;
  };
  deferred?: boolean;
  layer?: {
    group?: 'skin' | 'makeup';
    overEarlier?: boolean;
    paint: (sheet: Painting, values: number[], context: ToolContext) => void;
    blend: BlendMode | ((values: number[]) => BlendMode);
    opacity: (values: number[]) => number;
  };
  thumb?(photo: ImageLike, context: ToolContext): ImageLike | void;
};
export type ActionControl = {
  kind: 'action';
  key: string;
  label: string;
  apply?: (image: Image, context: ToolContext) => void;
  group?: string;
  repeatable?: boolean;
  tool?(photo: ImageLike, context: ToolContext): ImageLike | void;
  deferred?: boolean;
  thumb?(photo: ImageLike, context: ToolContext): ImageLike | void;
};
export type ToolControl<T = unknown> = {
  kind: 'tool';
  key: string;
  label: string;
  /** Selects an app-provided panel for this tool, when needed. */
  panel?: string;
  onApply?: (value?: unknown) => unknown;
  tool?(photo: ImageLike, value: T[], context: ToolContext): ImageLike | void;
  thumb?(photo: ImageLike, context: ToolContext): ImageLike | void;
};
/** A labeled container that groups child controls. */
export type GroupControl = { kind: 'group'; key: string; label: string; controls: EditControl[] };
/** A control that combines multiple child controls into a shared tool operation. */
export type MixedControl = {
  kind: 'mixed';
  key: string;
  label: string;
  controls: EditControl[];
  thumb?(photo: ImageLike, context: ToolContext): ImageLike | void;
  tool?(photo: ImageLike, values: number[][], context: ToolContext): ImageLike | void;
};
/** Image displayed for a picker option, either a bundled asset or a rendered preview. */
export type PickerPicture = { kind: 'asset'; source: unknown } | { kind: 'rendered' };
/** One selectable item in a picker control. */
export type PickerItem = {
  key: string;
  label: string;
  picture: PickerPicture;
  onTap?: () => void;
  value?: number;
  thumb?(photo: ImageLike, context: ToolContext): ImageLike | void;
};
/** Configuration for a one-choice or multi-choice picker control. */
export type PickerControl = {
  kind: 'picker';
  key: string;
  label: string;
  selection: 'one' | 'many';
  items: PickerItem[];
  commitTo?: { slider: string; index: number };
  after?: number;
};
/** Any supported plugin editor control. */
export type EditControl = SliderControl | ActionControl | ToolControl | GroupControl | MixedControl | PickerControl;
/** Entry point for a JavaScript plugin control script. */
export type ControlScript = () => EditControl;
