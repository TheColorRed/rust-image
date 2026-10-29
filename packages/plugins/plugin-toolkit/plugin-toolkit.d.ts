import type { AiModel } from '@abra/core';
import type { BlendMode } from '@abra/core/blend';
import type { CanvasLike } from '@abra/core/canvas';
import type { Rect } from '@abra/core/geometry';
import type { Image, ImageLike } from '@abra/core/image';
import type { MaskLike } from '@abra/core/mask';

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
  /** Add a file picker. Set `multiple` to allow multiple files and `thumbs` to show image thumbnails. */
  filePicker(label: string, options: { multiple?: boolean; thumbs?: boolean; onPick(files: PickedFile[]): void }): void;
  /** Adds a heading to the plugin editor inspector, like an <h1> to <h6>. `level` defaults to 1. */
  header(text: string, options?: { level?: 1 | 2 | 3 | 4 | 5 | 6 }): void;
  /** Adds two number fields side by side, like a size or a point. `value` defaults to `[0, 0]` and `labels` to `['Width', 'Height']`. `min` and `max` limit both numbers. `onChange` gets both numbers each time one is changed to a number. */
  size(
    label: string,
    options?: { value?: [number, number]; labels?: [string, string]; reset?: boolean; min?: number; max?: number; onChange?(value: [number, number]): void },
  ): void;
  /** Adds a bar of buttons to pick one of `options`, across the whole width. `value` defaults to the first. The editor runs the plugin again after `onChange`. */
  select(label: string, options: { options: string[]; value?: string; onChange?(value: string): void }): void;
  /** Adds an on/off switch. `value` defaults to false. The editor runs the plugin again after `onChange`, which gets whether it is on. */
  toggle(label: string, options?: { value?: boolean; onChange?(value: boolean): void }): void;
  /** Adds one number field.`value` defaults to `min` (or 0), `step` to 1, and `reset` to true. `min` and `max` limit the number. The editor runs the plugin again after `onChange`. */
  number(
    label: string,
    options?: { value?: number; min?: number; max?: number; step?: number; reset?: boolean; onChange?(value: number): void },
  ): void;
  /** Adds a slider to the plugin editor inspector. `min` defaults to 0, `step` to 1 and `range` to false (one thumb). */
  slider(
    label: string,
    options: { min?: number; max: number; step?: number; /** The name on a line of its own with the slider under it. Defaults to false, and to true when `range` is true. */ multiline?: boolean; /** A reset button beside the name, which puts the slider back to where it was when the editor opened. Defaults to true. */ reset?: boolean } & (
      | { range?: false; value?: number; onChange?(value: number): void }
      | { range: true; value?: [number, number]; onChange?(value: [number, number]): void }
    ),
  ): void;
};
/** One cell of a canvas made by `canvasOfCells`: the picture file that goes in it, and where the cell is and how big it is, in pixels of the canvas. */
export type PictureCell = {
  path: string;
  x: number;
  y: number;
  width: number;
  height: number;
  /** True keeps the picture's own proportions with nothing cropped: the cell shrinks to the largest picture that fits in `width` x `height` and sits in the middle of that place. False scales the picture to cover the cell and crops the rest. */
  keepAspect: boolean;
};
/**
 * A canvas of `width` x `height` with a child canvas for each of `cells`, in order, each holding its picture scaled to cover the cell
 * (the part that does not fit is cropped), at the cell's place. The pictures are read and scaled by the library on worker threads, in
 * parallel, and a file that more than one cell uses is read once, so the app keeps drawing while it works. The child canvases are the
 * canvas's `canvases()`, which can be turned, scaled and moved afterwards. Rejects with the file that could not be read. The plugin owns
 * the canvas and frees it with `release`.
 */
export declare function canvasOfCells(name: string, width: number, height: number, cells: PictureCell[]): Promise<CanvasLike>;
/** A random number generator that makes the same numbers every time it is given the same seed, the one the editor's effects use (bokeh, say), so what a plugin makes comes out the same when it is made again. Without a seed it is different each time. */
export declare class Rand {
  constructor(seed?: string);
  /** The next number, from 0 up to and not including 1. */
  next(): number;
}
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
    /** Prepare a model for use by the named owner and role. */
    prepareModel(owner: string, role: string): Promise<void>;
    /** Release models prepared by an owner, optionally limited to one role. */
    releaseModel(owner: string, role?: string): void;
    /** Run asynchronous work with a prepared model. */
    withModel<T>(owner: string, role: string, work: (model: AiModel) => Promise<T>): Promise<T>;
    /** Run a model file with a caller-supplied manifest and release the model afterward. */
    withManifest<T>(role: string, manifest: string, work: (model: AiModel) => Promise<T>): Promise<T>;
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
): Generator<{
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
