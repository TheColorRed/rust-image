import { selectedSkinMask, skinMaskSized } from '@/src/lib/analysis';
import { preloadFor } from '@/src/lib/editor-preloads';
import { withLayers, type Painting, type Sheet } from '@/src/lib/layers';
import { type BlendMode } from '@abra/core/blend';
import { type Image as AbraPixels, type ImageLike } from '@abra/core/image';
import { TransformAlgorithm, type MaskLike } from '@abra/core/mask';
import { applyEffect, type AbraImageLike, type EffectSpec } from '@alakazam/mobile';
import { adjustments } from './edit-sections/adjustments';
import { beauty } from './edit-sections/beauty';
import { blur } from './edit-sections/blur';
import { cleanup } from './edit-sections/cleanup';
import { detail } from './edit-sections/detail';
import { effects } from './edit-sections/effects';
import { BLEMISH_TOOL_KEY, RED_EYE_TOOL_KEY, SKIN_SMOOTH_KEY, SKIN_TAN_KEY, SKIN_TONE_KEY } from './edit-sections/keys';
import { colorization } from './edit-sections/mood';
import { sharpen } from './edit-sections/sharpen';
import { transform } from './edit-sections/transform';

/**
 * What a tool, a thumb or an `apply` is told besides the photo. The editor draws a photo shrunk to the screen and makes the
 * same edit on the full-size photo later, so the photo it gets is not always the same size.
 */
export type ToolContext = {
  /**
   * How much smaller the photo is than the full-size one: 1 for the full-size photo (or one that already fits the screen), and
   * less than 1 for a shrunk copy, such as 0.25 for a copy a quarter as wide. Multiply a size given in pixels (a blur radius,
   * a sharpening distance) by it, so the edit looks the same at every size. A size given as a fraction of the photo needs no
   * change.
   */
  scale: number;
  /**
   * What the control's `preload` made: masks or other images the tool needs but should not make on every call. It is the
   * editor's to free, so a tool must not destroy it. It is the array or the object the `preload` returned, one image or mask
   * becomes an array of one, and nothing an empty array. Missing for a control with no `preload`, and for a thumb.
   */
  preloaded?: Preloaded;
  /**
   * The box of the one person the edit is for, in pixels of the photo the tool is given, for a control that can be set for one
   * person. Missing when the edit is for everyone.
   */
  person?: { x: number; y: number; width: number; height: number; id?: number };
  /**
   * Makes the mask of the skin the editor found, of the person the edit is for or of everyone, at the size of the photo the tool
   * is given. It is nothing while the skin has not been found. Each call makes a new mask that the tool must free.
   */
  skin?: () => MaskLike | undefined;
  /**
   * The ids of the people who have a value of their own for this control, for the layer that is for everyone. Their skin and
   * faces are left out of it, so their own value replaces this one for them. Missing for a layer that is for one person.
   */
  others?: number[];
};

/** What a `preload` makes, as the tool gets it: an array, or an object with a name for each image or mask. */
export type Preloaded = DataLike[] | Record<string, DataLike>;

/**
 * The mask of the skin that the editor found, of the person with this id or of everyone, resized to `photo`. Nothing while the
 * skin has not been found. The caller frees it.
 */
export const skinMaskFor = (
  person: number | undefined,
  photo: { width(): number; height(): number },
  others?: number[],
): MaskLike | undefined => {
  return skinMaskSized(person, photo.width(), photo.height(), others);
};

/** How much smaller `image` is than `full`, from 0 to 1: 1 when it is as wide, or when there is no full-size photo to compare. */
export const scaleBetween = (image: { width(): number }, full: { width(): number } | null | undefined): number => {
  const fullWidth = full?.width() ?? 0;
  return fullWidth > 0 ? Math.min(1, image.width() / fullWidth) : 1;
};

/**
 * One setting, or one for each slider of a control that has several. In an array an item may be left out (`[, 1]`) or be
 * `undefined`, and that slider then takes the default for the setting.
 */
export type PerSlider<T> = T | (T | undefined)[];

/** What a slider control has committed: a number for a control with one slider, an array of numbers (one each) for several. */
export type AdjustmentValue = number | number[];

export type DataLike = ImageLike | MaskLike;

/**
 * A control whose value is applied once the user releases the slider.
 *
 * A control has several sliders when `sliderLabel`, `min`, `max`, `step` or `defaultValue` is an array: each item is one slider, and
 * the arrays must be the same length. A setting given once (`step: 1`) is the same for every slider, and an item left out of an
 * array (`min: [, 1]`) takes the default for that setting. The control's committed
 * value is then an array with one number for each slider, and its `tool` gets that array whenever any one slider moves.
 */
export type SliderControl<T = unknown> = {
  /** A slider control allows the user to select a value within a range by dragging a slider. */
  kind: 'slider';
  /** How to display multiple sliders: 'all' shows all sliders at once, 'tabulated' shows them in a table-like layout. */
  display?: 'all' | 'tabulated';
  /** The unique key identifying this control. */
  key: string;
  /** The main label for the control. */
  label: string;
  /** The label displayed for each individual slider, if different from the main label. */
  sliderLabel?: PerSlider<string>;
  /** The minimum value each slider can take. Defaults to 0 if not specified. */
  min?: PerSlider<number>;
  /** The maximum value each slider can take. Defaults to 100 if not specified. */
  max?: PerSlider<number>;
  /** The increment by which each slider value changes. Defaults to 1 if not specified. */
  step?: PerSlider<number>;
  /**
   * The value each slider snaps back to after each release; each drag applies a fresh delta.
   *  Defaults to 0 if not specified.
   */
  defaultValue?: PerSlider<number>;
  /**
   * Whether selecting the slider applies `defaultValue` at once, and the control counts as applied whenever it has a value.
   * On by default, for effects with no neutral value (Threshold has no "off" setting). Set it to `false` when `defaultValue`
   * means "no change" (Brightness at 0): the control then counts as applied only once it differs from `defaultValue`.
   */
  applyOnSelect?: boolean;
  /**
   * Applies the slider's effect to an image. Without it, replaying the slider applies its `live` effect, which is the same
   * definition. Only for a control with one slider: use `tool` for several.
   *
   * @deprecated - Use `tool` instead: it is written against the Abra library, supports several sliders, and is the one definition of the control.
   */
  apply?: (image: AbraImageLike, value: number, context: ToolContext) => void;
  /**
   * A Vessel component to show in place of the plain slider, for a slider whose value is better picked another way; one for
   * each slider of a control that has several. The value is still 0 to 1 on the slider's scale. 'skin-tan' is the bar of tans;
   * 'hue' is the bar of colors and 'shade' the bar of shadow shades (a light tan to a dark
   * brown), which only pick.
   */
  picker?: PerSlider<'skin-tan' | 'hue' | 'shade'>;
  /**
   * Ready-made choices shown above a picker, for people who would not know which sliders to move to get a color (black is no
   * brightness). Tapping one sets the sliders it lists, in `values` (one for each slider; `undefined` leaves a slider alone),
   * and applies the control. `color` is the swatch's CSS color.
   */
  swatches?: { label: string; color: string; values: (number | undefined)[] }[];
  /**
   * When set, dragging previews this effect live (GPU where available) before the release commits it via `apply`, or this same
   * effect when there is no `apply`. Only for a control with one slider: use `tool` for several.
   *
   * @deprecated - Use `tool` instead: it is written against the Abra library and supports several sliders; `live` is limited to one slider and the effects that have a shader form.
   */
  live?: (value: number) => EffectSpec;
  /**
   * Runs something when the control is about to be used.
   * Useful for doing things that need to be prepared before the effect is applied.
   * This could be something like generating a mask or precomputing values that don't change frequently.
   * @param photo The image to preload the effect on.
   * @returns An image or mask, an array of them, an object with a name for each (`{ highlightMask, shadowMask }`), or nothing.
   * They are handed to `tool` as `context.preloaded` in the same shape, and freed by the editor.
   */
  preload?: (photo: ImageLike) => void | DataLike | DataLike[] | Record<string, DataLike>;
  /**
   * A tool written against the Abra library (`@abra/core`). It gets an image that is its own to change: it can change it in
   * place and return nothing, which makes the changed image the result, or return a new image, which is then the result and
   * the image it was given is thrown away. It runs on a screen-sized copy while the slider is dragged and on the full image
   * when the edit is replayed, so it is the one definition of the control. Use it for what the library can do and the effects
   * (`live`, deprecated) cannot yet.
   *
   * It gets the control's values as an array of `T`, which is `unknown` unless the control says otherwise, so a tool declares
   * the type it knows in its own parameters: `tool: (photo, [strength]: number[]) => vignette(photo, strength / 100)`. This is a
   * method so that a tool may declare a narrower parameter type than the control's `T`.
   */
  tool?(photo: ImageLike, value: T[], context: ToolContext): ImageLike | void;
  /**
   * Puts the control on a layer of its own over the photo, in the editor's canvas, instead of drawing on the photo, and replaces
   * `tool`. `paint` gets the sheet (which knows the size of the photo) and paints the color where it goes with `paintSheet`; the
   * sheet is cut down to the part that was painted. The sheet is laid over the photo with `blend`, shown at `opacity`, so the
   * strength is a layer setting. Used by makeup.
   */
  layer?: {
    /** The group canvas the layer goes in. Makeup when not given. */
    group?: 'skin' | 'makeup';
    /** Paints the sheet from the control's values, with the same context a tool gets. */
    paint: (sheet: Painting, values: number[], context: ToolContext) => void;
    /** How the sheet is laid over what is under it, or a function of the control's values that picks it. */
    blend: BlendMode | ((values: number[]) => BlendMode);
    /** How much of the sheet shows, 0 to 1, from the control's values. */
    opacity: (values: number[]) => number;
  };
  /**
   * Makes the control's thumbnail, in sections that show one. It gets a thumbnail-size copy of the photo, which is its own to
   * change: it changes it in place and returns nothing, or returns a new image to use instead. A slider has no single value
   * to show, so it gets only the image (and the context, whose `scale` is small here) and picks what to illustrate. Used in
   * place of `live` (deprecated) when it is set.
   */
  thumb?(photo: ImageLike, context: ToolContext): ImageLike | void;
};

/** A control that applies immediately on tap, with no parameters. */
export type ActionControl = {
  kind: 'action';
  key: string;
  label: string;
  /**
   * Applies the action's effect to an image. Without it, replaying the action applies its `tool` or `live` effect.
   *
   * @deprecated - Use `tool` instead: it is written against the Abra library and is the one definition of the control.
   */
  apply?: (image: AbraImageLike, context: ToolContext) => void;
  /**
   * Controls sharing a group are mutually exclusive: applying one clears any other applied
   * control in the same group (across all sections). Ungrouped controls toggle independently.
   */
  group?: string;
  /**
   * The effects this action applies, as one effect or several that run in order. Tapping previews them at once in a live
   * image while the full-resolution edit replays behind it; effects without a shader run on the CPU at preview size.
   * Without `apply`, replaying the action applies these same effects, so there is one definition of the action.
   *
   * @deprecated - Use `tool` instead: it is written against the Abra library and can do what the effects cannot.
   */
  live?: () => EffectSpec | EffectSpec[];
  /**
   * A tool written against the Abra library (`@abra/core`), the same as a slider's: it gets an image that is its own to
   * change, and either changes it in place and returns nothing, or returns a new image to use instead. An action has no
   * value, so it gets only the image. It runs on the full image when the edit is replayed; there is no instant preview of it
   * yet, as there is for `live` (deprecated).
   */
  tool?(photo: ImageLike, context: ToolContext): ImageLike | void;
  /**
   * Makes the action's thumbnail, in sections that show one. It gets a thumbnail-size copy of the photo, which is its own to
   * change: it changes it in place and returns nothing, or returns a new image to use instead. Used in place of `live` when it
   * is set (`live` is deprecated), so a control whose `tool` has no effect form can still show what it does.
   */
  thumb?(photo: ImageLike, context: ToolContext): ImageLike | void;
};

/** An effect or a list of effects as a list. */
export const effectList = (effects: EffectSpec | EffectSpec[]): EffectSpec[] =>
  Array.isArray(effects) ? effects : [effects];

/**
 * A control with no value of its own to configure — tapping it hands off to a custom, on-image
 * interaction of the edit screen's own choosing (e.g. the blemish tool's reticle + button, or a
 * future gradient tool's tap-two-points-then-apply), rather than a slider or an instant whole-
 * image effect. Unlike `ActionControl` it has no `apply`, since what it does isn't reusable across
 * controls — the screen dispatches on `key` to render and run the right one.
 */
export type ToolControl<T = unknown> = {
  kind: 'tool';
  key: string;
  label: string;
  /**
   * A tool written against the Abra library (`@abra/core`), the same as a slider's: it gets an image that is its own to
   * change, and either changes it in place and returns nothing, or returns a new image to use instead. It also gets the
   * values the on-image interaction produced, as an array of `T` (for example the points the user tapped), which is
   * `unknown` unless the control says otherwise.
   */
  tool?(photo: ImageLike, value: T[], context: ToolContext): ImageLike | void;
  /**
   * Makes the control's thumbnail, in sections that show one. It gets a thumbnail-size copy of the photo, which is its own to
   * change: it changes it in place and returns nothing, or returns a new image to use instead.
   */
  thumb?(photo: ImageLike, context: ToolContext): ImageLike | void;
};

/**
 * Applies an action to `image`: its own `apply`, or else its tool, or else its live effect, which is the same definition.
 * The `apply` and `live` paths are deprecated (see `ActionControl.apply` and `ActionControl.live`); use `tool`.
 */
export const applyAction = (control: ActionControl, image: AbraImageLike, scale = 1) => {
  const context: ToolContext = { scale };
  if (control.apply) control.apply(image, context);
  else if (control.tool) {
    const photo = image.clone();
    // The tool changed `photo` in place, or made a new image to use instead.
    const returned = control.tool(photo, context) as ImageLike | undefined;
    image.replaceWith(returned ?? photo);
    // They are native images the garbage collector cannot see the size of.
    (photo as AbraPixels).uniffiDestroy();
    if (returned) (returned as AbraPixels).uniffiDestroy();
  } else if (control.live) for (const effect of effectList(control.live())) applyEffect(image, effect, selectedSkinMask());
};

/** One slider of a control, with every setting filled in. */
export type SliderSpec = { label: string; min: number; max: number; step: number; defaultValue: number };

/** Whether the control has several sliders, which is when any of its per-slider settings is an array. */
export const isMultiSlider = (control: SliderControl): boolean =>
  [control.sliderLabel, control.min, control.max, control.step, control.defaultValue].some(Array.isArray);

/**
 * The sliders of a control, one for each item of its arrays, or the one slider of a control that has none. Throws when the
 * arrays are not all the same length.
 */
export const sliderSpecs = (control: SliderControl): SliderSpec[] => {
  const settings = [control.sliderLabel, control.min, control.max, control.step, control.defaultValue];
  const lengths = settings.flatMap(setting => (Array.isArray(setting) ? [setting.length] : []));
  const count = lengths[0] ?? 1;
  if (lengths.some(length => length !== count)) {
    throw new Error(
      `The slider control "${control.key}" needs its sliderLabel, min, max, step and defaultValue arrays to be the same length`,
    );
  }
  // A setting that is not given, or is left out of an array, is the default. A slider with no label of its own takes the control's.
  const at = <V>(setting: PerSlider<V> | undefined, index: number, fallback: V): V =>
    (Array.isArray(setting) ? setting[index] : setting) ?? fallback;
  return Array.from({ length: count }, (_, index) => ({
    label: at(control.sliderLabel, index, control.label),
    min: at(control.min, index, 0),
    max: at(control.max, index, 100),
    step: at(control.step, index, 1),
    defaultValue: at(control.defaultValue, index, 0),
  }));
};

/** What a control's value is while it has not been changed: a number for one slider, one number for each of several. */
export const sliderDefault = (control: SliderControl): AdjustmentValue => {
  const defaults = sliderSpecs(control).map(spec => spec.defaultValue);
  return isMultiSlider(control) ? defaults : defaults[0];
};

/** The value of each slider, taken from the committed `value` and from the default where there is none. */
export const sliderValues = (control: SliderControl, value?: AdjustmentValue): number[] => {
  const defaults = sliderSpecs(control).map(spec => spec.defaultValue);
  const committed = value === undefined ? [] : Array.isArray(value) ? value : [value];
  return defaults.map((fallback, index) => committed[index] ?? fallback);
};

/** What a slider's tool is told besides the photo, when it is applied to `image` (the photo, or a copy of it at the same size). */
export const sliderContext = (
  control: SliderControl,
  image: AbraImageLike,
  scale: number,
  person?: ToolContext['person'],
  others?: number[],
): ToolContext => ({
  scale,
  preloaded: control.preload ? preloadFor(control, image.width(), image) : undefined,
  person,
  others,
  skin: () => skinMaskFor(person?.id, image, others),
});

/** The sheet a control with a `layer` paints, for these slider values. `name` is the layer's name: the control's key, or its key for one person. */
export const layerSheet = (control: SliderControl, values: number[], context: ToolContext, name = control.key): Sheet => {
  const { layer } = control;
  if (!layer) throw new Error(`The control "${control.key}" has no layer to paint`);
  return {
    name,
    group: layer.group,
    blend: typeof layer.blend === 'function' ? layer.blend(values) : layer.blend,
    opacity: layer.opacity(values),
    paint: sheet => layer.paint(sheet, values, context),
  };
};

/**
 * Applies a slider at `value` to `image`: its own `apply`, or else its tool, or else its live effect.
 * The `apply` and `live` paths are deprecated (see `SliderControl.apply` and `SliderControl.live`); use `tool`.
 */
export const applySlider = (
  control: SliderControl,
  image: AbraImageLike,
  value: AdjustmentValue,
  scale = 1,
  person?: ToolContext['person'],
) => {
  const values = sliderValues(control, value);
  const context = sliderContext(control, image, scale, person);
  if (control.layer) {
    const laid = withLayers(image, [layerSheet(control, values, context)]);
    image.replaceWith(laid);
    (laid as AbraPixels).uniffiDestroy();
  } else if (control.apply) control.apply(image, values[0], context);
  else if (control.tool) {
    const photo = image.clone();
    // The tool changed `photo` in place, or made a new image to use instead.
    const returned = control.tool(photo, values, context) as ImageLike | undefined;
    image.replaceWith(returned ?? photo);
    // They are native images the garbage collector cannot see the size of.
    (photo as AbraPixels).uniffiDestroy();
    if (returned) (returned as AbraPixels).uniffiDestroy();
  } else if (control.live) applyEffect(image, control.live(values[0]), selectedSkinMask());
};

/** Whether a slider's effect is on, given its committed value (undefined when it has none). */
export const appliesOnSelect = (control: SliderControl) => control.applyOnSelect !== false;

export const isSliderApplied = (control: SliderControl, value: AdjustmentValue | undefined) =>
  appliesOnSelect(control)
    ? value !== undefined
    : sliderValues(control, value).some((slider, index) => slider !== sliderSpecs(control)[index].defaultValue);

/**
 * A control that opens a row of controls of its own, one level down: tapping it replaces the section's row with these, with a
 * way back. The controls inside may be any kind, so a group may hold groups.
 */
export type GroupControl = {
  kind: 'group';
  key: string;
  label: string;
  controls: EditControl[];
};

export type EditControl = SliderControl | ActionControl | ToolControl | GroupControl;

export type EditSection = {
  key: string;
  label: string;
  controls: EditControl[];
  /** Show each control as a thumbnail of the effect applied, instead of a plain text chip. */
  previewThumbnails?: boolean;
};

/** Key of the blemish tool control; the editor dispatches on it to render the reticle and controls. */
export { BLEMISH_TOOL_KEY, RED_EYE_TOOL_KEY, SKIN_SMOOTH_KEY, SKIN_TAN_KEY, SKIN_TONE_KEY };

// Ordered by what a casual, non-designer social-media poster reaches for first: filters, then
// basic brightness/color tweaks, then face retouching — with the more technical, restoration-style
// tools (Detail, Cleanup) pushed to the back since that audience rarely needs them.
export const EDIT_SECTIONS: EditSection[] = [
  colorization,
  adjustments,
  beauty,
  transform,
  sharpen,
  effects,
  blur,
  detail,
  cleanup,
];

/** The controls that do something, with every group opened out into the controls inside it. */
export const leafControls = (controls: EditControl[]): Exclude<EditControl, GroupControl>[] =>
  controls.flatMap(control => (control.kind === 'group' ? leafControls(control.controls) : [control]));

/**
 * The groups to open, outermost first, to reach the group with this key at any depth: the last is that group and the one before
 * it is the group to go back to. Empty when there is no such group.
 */
export const groupPath = (controls: EditControl[], key: string): GroupControl[] => {
  for (const control of controls) {
    if (control.kind !== 'group') continue;
    if (control.key === key) return [control];
    const inside = groupPath(control.controls, key);
    if (inside.length) return [control, ...inside];
  }
  return [];
};

export const findSection = (key: string): EditSection =>
  EDIT_SECTIONS.find(section => section.key === key) ?? EDIT_SECTIONS[0];
