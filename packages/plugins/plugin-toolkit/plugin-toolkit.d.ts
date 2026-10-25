import type { AiModel } from '@abra/core';
import type { BlendMode } from '@abra/core/blend';
import type { ColorLike } from '@abra/core/color';
import type { Image, ImageLike } from '@abra/core/image';
import type { MaskLike } from '@abra/core/mask';

/** Set temporary text for the app's AI pending notice. Set to null to restore its generic text. */
export declare function setAiPendingNotice(value: AiPendingNoticeText | null): void;
/** Prefix a plugin-owned control key with the current plugin ID. */
export declare function toolKey(id: string): string;
export type AiPendingNoticeText = { text: string; subText?: string; detail?: string };
export type AppModel = { id: string; title: string; description: string; enabled: boolean; kinds: string[]; downloaded: boolean };
export declare const app: {
  editor: {
    currentImage(): ImageLike | null;
    hasHistory(): boolean;
    flush(): void;
    area(key: string): number[][][];
    setArea(key: string, rings: number[][][]): void;
    setBusy(value: boolean): void;
    replaceImage(image: ImageLike, result: ImageLike): void;
  };
  ai: {
    models(): readonly AppModel[];
    selectedModel(key: string): string | null;
    setSelectedModel(key: string, role: string): void;
    missing(roles: readonly string[]): string[];
    requestModels(title: string, roles: readonly string[], onReady: () => void): void;
    prepareModel(owner: string, role: string): Promise<void>;
    releaseModel(owner: string, role?: string): void;
    withModel<T>(owner: string, role: string, work: (model: AiModel) => Promise<T>): Promise<T>;
    runGuided(model: AiModel, image: ImageLike, mask: MaskLike): Promise<{ image(): ImageLike }>;
    withProgress<T>(key: string, work: () => Promise<T>): Promise<T>;
  };
  history: {
    commitRetouch(change: { type: string; [key: string]: unknown }): void;
    save(): Promise<void>;
  };
  tools: { setMessage(key: string, message: string | null): void };
};
export declare function faceMasks(photo: ImageLike): Promise<Record<string, MaskLike> | undefined>;
export declare function areaMask(
  rings: number[][][], width: number, height: number, keep: Keep,
): MaskLike;
export declare function belongsTo(
  box: string,
  photo: { width(): number; height(): number },
  person: { x: number; y: number; width: number; height: number } | undefined,
  others?: number[],
): boolean;
export declare function derivedFor(
  owner: Preloaded,
  group: string,
  variant: string,
  make: () => DataLike | undefined,
): DataLike | undefined;
export type Keep = {
  <V>(value: V): V;
  <V extends unknown[]>(...values: V): V;
};
export declare function owning<T>(work: (keep: Keep) => T): T;
export declare function release(...values: unknown[]): void;
export declare function paintSheet(sheet: Painting, regions: MaskLike[], color: Color): void;
export declare function swatchesOf(
  colors: { label: string; color: string }[],
  values: (hue: number, saturation: number, brightness: number, label: string) => (number | undefined)[],
): { label: string; color: string; values: (number | undefined)[] }[];

export type ToolContext = {
  scale: number;
  preloaded?: Preloaded;
  areas?: (number[][][] | undefined)[];
  person?: { x: number; y: number; width: number; height: number; id?: number };
  skin?: () => MaskLike | undefined;
  others?: number[];
  values?: number[];
};
export type DataLike = ImageLike | MaskLike;
export type Preloaded = DataLike[] | Record<string, DataLike>;
export type Preloading = void | DataLike | DataLike[] | Record<string, DataLike> | Promise<void | DataLike | DataLike[] | Record<string, DataLike>>;
export type PerSlider<T> = T | (T | undefined)[];
export type RangeSetting = number | [number, number];
export type AdjustmentValue = number | number[];
export type PillFormat = 'signed' | 'percent' | 'plain';
export type Painting = {
  photo: ImageLike;
  width(): number;
  height(): number;
  painted?: { image: ImageLike; x: number; y: number };
};

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
  joint?: { key: string; sliders: SliderControl[]; tool(photo: ImageLike, values: number[][], context: ToolContext): ImageLike | void };
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
  kind: 'action'; key: string; label: string;
  apply?: (image: Image, context: ToolContext) => void;
  group?: string; repeatable?: boolean;
  tool?(photo: ImageLike, context: ToolContext): ImageLike | void;
  deferred?: boolean;
  thumb?(photo: ImageLike, context: ToolContext): ImageLike | void;
};
export type ToolControl<T = unknown> = {
  kind: 'tool'; key: string; label: string;
  /** Selects an app-provided panel for this tool, when needed. */
  panel?: string;
  onApply?: () => void;
  tool?(photo: ImageLike, value: T[], context: ToolContext): ImageLike | void;
  thumb?(photo: ImageLike, context: ToolContext): ImageLike | void;
};
export type GroupControl = { kind: 'group'; key: string; label: string; controls: EditControl[] };
export type MixedControl = {
  kind: 'mixed'; key: string; label: string; controls: EditControl[];
  thumb?(photo: ImageLike, context: ToolContext): ImageLike | void;
  tool?(photo: ImageLike, values: number[][], context: ToolContext): ImageLike | void;
};
export type PickerPicture = { kind: 'asset'; source: unknown } | { kind: 'rendered' };
export type PickerItem = {
  key: string; label: string; picture: PickerPicture; onTap?: () => void; value?: number;
  thumb?(photo: ImageLike, context: ToolContext): ImageLike | void;
};
export type PickerControl = {
  kind: 'picker'; key: string; label: string; selection: 'one' | 'many'; items: PickerItem[];
  commitTo?: { slider: string; index: number }; after?: number;
};
export type EditControl = SliderControl | ActionControl | ToolControl | GroupControl | MixedControl | PickerControl;
export type ControlScript = () => EditControl;
