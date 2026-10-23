import { setControlTraits } from '@/src/state/control-traits';
import { type ImageSourcePropType } from 'react-native';
import { skinMaskSized } from '@/src/lib/ai/analysis';
import { preloadFor } from '@/src/lib/editor/editor-preloads';
import { withLayers, type Painting, type Sheet } from '@/src/lib/beauty/layers';
import { type BlendMode } from '@abra/core/blend';
import { type Image as AbraPixels, type ImageLike } from '@abra/core/image';
import { type MaskLike } from '@abra/core/mask';
import { type AbraImageLike } from '@alakazam/mobile';
import { adjustments } from '@/src/lib/project/edit-sections/adjustments';
import { beauty } from '@/src/lib/project/edit-sections/beauty';
import { blur } from '@/src/lib/project/edit-sections/blur';
import { cleanup } from '@/src/lib/project/edit-sections/cleanup';
import { detail } from '@/src/lib/project/edit-sections/detail';
import { effects } from '@/src/lib/project/edit-sections/effects';
import { BLEMISH_TOOL_KEY, CROP_TOOL_KEY, RED_EYE_TOOL_KEY, SKIN_SMOOTH_KEY, SKIN_TAN_KEY, SKIN_TONE_KEY } from '@/src/lib/project/edit-sections/keys';
import { colorization } from '@/src/lib/project/edit-sections/mood';
import { sharpen } from '@/src/lib/project/edit-sections/sharpen';
import { specialEffects } from '@/src/lib/project/edit-sections/special-effects';
import { transform } from '@/src/lib/project/edit-sections/transform';
import { release } from '@/src/lib/native/native-handles';

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
   * becomes an array of one, and nothing an empty array. Missing for a control with no `preload`, and for a thumb when what the
   * `preload` makes has not been made yet.
   */
  preloaded?: Preloaded;
  /**
   * The closed outlines drawn for each slider of a control that has `areaKeys`, in slider order. Each area is a list of rings, and
   * each ring a list of `[x, y]` points, each a fraction of the photo from 0 to 1. Missing for a slider with no area, which means
   * the whole photo.
   */
  areas?: (number[][][] | undefined)[];
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
  /**
   * For the `thumb` of a picker item: the values the slider the picker saves to holds now, with what the pickers saved after its
   * own, so a picture changes only its own part and shows the choices made in the other tabs. Missing otherwise.
   */
  values?: number[];
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

/**
 * A setting of one slider that may be given as a pair `[low, high]`: a pair makes the slider a range slider, with two thumbs
 * that pick a low and a high value. `defaultValue` being a pair is what makes it a range; `min`, `max` and `step` then may be
 * a pair too, one for each thumb, or a single number for both.
 */
export type RangeSetting = number | [number, number];

/** What a slider control has committed: a number for a control with one slider, an array of numbers (one each) for several. */
export type AdjustmentValue = number | number[];

export type DataLike = ImageLike | MaskLike;

/** What a `preload` can give back, before it is `Preloaded`: one image or mask, several, or nothing. */
export type Preloading = void | DataLike | DataLike[] | Record<string, DataLike>;

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
  /**
   * How each slider's pill shows its value when `display` is 'tabulated': 'signed' as `Warmth (+0.5)`, 'percent' as `Warmth 50%`,
   * or 'plain' as `Range · 40`. Defaults to 'signed'. A slider with a `picker` shows its color on the pill instead.
   */
  pillFormat?: PerSlider<PillFormat>;
  /**
   * Words to show in place of a slider's number, for the values that mean something other than a quantity (`Random` at the top
   * of a rotation). It gets the value and returns the text, or nothing to show the number. Used by the readout and the pill.
   */
  valueText?: PerSlider<(value: number) => string | undefined>;
  /** The unique key identifying this control. */
  key: string;
  /** The main label for the control. */
  label: string;
  /** Whether the control can be set for one person, or for everyone. A value for one person is kept under its own key (see `adjustmentKey`). */
  person?: boolean;
  /** Whether the control's work is confined to a face, so opening it brings the selected person's face closer. */
  face?: boolean;
  /**
   * The AI models the control cannot work without, by id, and the name to offer them under. Opening the control without them offers to
   * download them first, and opens it once they are here.
   */
  needsModels?: { title: string; roles: readonly string[] };
  /** The label displayed for each individual slider, if different from the main label. */
  sliderLabel?: PerSlider<string>;
  /** The minimum value each slider can take. Defaults to 0 if not specified. */
  min?: PerSlider<RangeSetting>;
  /** The maximum value each slider can take. Defaults to 100 if not specified. */
  max?: PerSlider<RangeSetting>;
  /** The increment by which each slider value changes. Defaults to 1 if not specified. */
  step?: PerSlider<RangeSetting>;
  /**
   * The value each slider snaps back to after each release; each drag applies a fresh delta.
   *  Defaults to 0 if not specified. A pair makes a range slider: `defaultValue: [0, 0, [2, 8]]` has two sliders and a range
   * slider. A range slider takes two places in the control's value, its low value and then its high value, and in a tabulated
   * control it is one tab.
   */
  defaultValue?: PerSlider<RangeSetting>;
  /**
   * The adjustment keys holding a drawn area for each slider, in slider order (see `colorAreaKey`). Each is a flat list of x and y
   * fractions of the photo, from 0 to 1, with rings separated by `AREA_RING_BREAK` pairs (see `areaPoints`). The area reaches the
   * tool as `context.areas`; a slider with no area has none there.
   */
  areaKeys?: string[];
  /**
   * Whether selecting the slider applies `defaultValue` at once, and the control counts as applied whenever it has a value.
   * On by default, for effects with no neutral value (Threshold has no "off" setting). Set it to `false` when `defaultValue`
   * means "no change" (Brightness at 0): the control then counts as applied only once it differs from `defaultValue`.
   */
  applyOnSelect?: boolean;
  /**
   * Applies the slider's effect to an image. Without it, replaying the slider applies its `tool`. Only for a control with one
   * slider: use `tool` for several.
   *
   * @deprecated - Use `tool` instead: it is written against the Abra library, supports several sliders, and is the one definition of the control.
   */
  apply?: (image: AbraImageLike, value: number, context: ToolContext) => void;
  /**
   * A Vessel component to show in place of the plain slider, for a slider whose value is better picked another way; one for
   * each slider of a control that has several. The value is still 0 to 1 on the slider's scale. 'skin-tan' is the bar of tans;
   * 'hue' is the bar of colors and 'shade' the bar of shadow shades (a light tan to a dark
   * brown) and 'glow' the bar of highlighter tints (pearl to rose) and 'infrared' the bar of infrared tints (hot pink to white) and 'neon' the bar of neon tints (teal to hot pink),
   * which only pick. 'swatches' has no bar: the slider is chosen from the control's `swatches` alone, so a control can offer only a set of
   * named colors.
   */
  picker?: PerSlider<'skin-tan' | 'hue' | 'shade' | 'glow' | 'infrared' | 'neon' | 'swatches'>;
  /**
   * Ready-made choices shown above a picker, for people who would not know which sliders to move to get a color (black is no
   * brightness). Tapping one sets the sliders it lists, in `values` (one for each slider; `undefined` leaves a slider alone),
   * and applies the control. `color` is the swatch's CSS color.
   */
  swatches?: { label: string; color: string; values: (number | undefined)[] }[];
  /**
   * Runs something when the control is about to be used.
   * Useful for doing things that need to be prepared before the effect is applied.
   * This could be something like generating a mask or precomputing values that don't change frequently.
   * @param photo The image to preload the effect on.
   * @returns An image or mask, an array of them, an object with a name for each (`{ highlightMask, shadowMask }`), or nothing.
   * They are handed to `tool` as `context.preloaded` in the same shape, and freed by the editor.
   *
   * A preload that is slow (an AI model) returns a promise and does its work in the background: the editor goes on, `tool` gets no
   * `preloaded` until the promise is done, and the editor draws again then. It must copy `photo` before it awaits, because the editor
   * frees `photo` when the function returns.
   */
  preload?: (photo: ImageLike) => Preloading | Promise<Preloading>;
  /**
   * A tool written against the Abra library (`@abra/core`). It gets an image that is its own to change: it can change it in
   * place and return nothing, which makes the changed image the result, or return a new image, which is then the result and
   * the image it was given is thrown away. It runs on a screen-sized copy while the slider is dragged and on the full image
   * when the edit is replayed, so it is the one definition of the control. Use it for what the library can do and the effects
   * old `apply` cannot.
   *
   * It gets the control's values as an array of `T`, which is `unknown` unless the control says otherwise, so a tool declares
   * the type it knows in its own parameters: `tool: (photo, [strength]: number[]) => vignette(photo, strength / 100)`. This is a
   * method so that a tool may declare a narrower parameter type than the control's `T`.
   */
  tool?(photo: ImageLike, value: T[], context: ToolContext): ImageLike | void;
  /**
   * Set by `leafControls` on the sliders of a mixed control with a `tool`: that tool, and the sliders it is made from, which
   * are applied together as one edit.
   */
  joint?: {
    key: string;
    sliders: SliderControl[];
    tool(photo: ImageLike, values: number[][], context: ToolContext): ImageLike | void;
  };
  /**
   * Set when the `tool` only applies library effects with `applyDeferred`, so it changes no pixels: the photo remembers the
   * effects and the GPU does them when it draws, for the part in view. The edit can then be made on the full-size photo at no
   * cost, which is what the editor draws, so zooming in is sharp. A tool that needs the CPU (masks, people, layers) leaves it off
   * and is made on a screen-size copy.
   */
  deferred?: boolean;
  /**
   * Puts the control on a layer of its own over the photo, in the editor's canvas, instead of drawing on the photo, and replaces
   * `tool`. `paint` gets the sheet (which knows the size of the photo) and paints the color where it goes with `paintSheet`; the
   * sheet is cut down to the part that was painted. The sheet is laid over the photo with `blend`, shown at `opacity`, so the
   * strength is a layer setting. Used by makeup.
   */
  layer?: {
    /** The group canvas the layer goes in. Makeup when not given. */
    group?: 'skin' | 'makeup';
    /** The sheet is cut from the photo and laid opaque over the skin (tone, smooth), so it must be cut from the photo with the earlier layers laid, or it would cover them. */
    overEarlier?: boolean;
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
   * place of the control's `tool` when it is set.
   */
  thumb?(photo: ImageLike, context: ToolContext): ImageLike | void;
};

/** A control that applies immediately on tap, with no parameters. */
export type ActionControl = {
  kind: 'action';
  key: string;
  label: string;
  /**
   * Applies the action's effect to an image. Without it, replaying the action applies its `tool`.
   *
   * @deprecated - Use `tool` instead: it is written against the Abra library and is the one definition of the control.
   */
  apply?: (image: AbraImageLike, context: ToolContext) => void;
  /**
   * Controls sharing a group are mutually exclusive: applying one clears any other applied
   * control in the same group (across all sections). Ungrouped controls toggle independently.
   */
  group?: string;
  /** Each tap adds another application of the action instead of toggling it off. Undo steps it back. */
  repeatable?: boolean;
  /**
   * A tool written against the Abra library (`@abra/core`), the same as a slider's: it gets an image that is its own to
   * change, and either changes it in place and returns nothing, or returns a new image to use instead. An action has no
   * value, so it gets only the image. It runs on the preview while the action is chosen and on the full image when the edit
   * is replayed, so it is the one definition of the action.
   */
  tool?(photo: ImageLike, context: ToolContext): ImageLike | void;
  /** Set when the `tool` only applies library effects with `applyDeferred`: see `SliderControl.deferred`. */
  deferred?: boolean;
  /**
   * Makes the action's thumbnail, in sections that show one. It gets a thumbnail-size copy of the photo, which is its own to
   * change: it changes it in place and returns nothing, or returns a new image to use instead. Used in place of the action's
   * `tool` when it is set, so a control whose `tool` is too slow for a thumbnail can still show what it does.
   */
  thumb?(photo: ImageLike, context: ToolContext): ImageLike | void;
};

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
 * A control that opens a row of controls of its own, one level down: tapping it replaces the section's row with these, with a
 * way back. The controls inside may be any kind, so a group may hold groups.
 */
export type GroupControl = {
  kind: 'group';
  key: string;
  label: string;
  controls: EditControl[];
};

/**
 * A row of controls of mixed kinds (slider, action, picker, ...): each control is one tab of the row, shown the way a tabulated
 * slider's sliders are. Tapping a tab opens that control. Its controls are part of the section, so they are found by `leafControls`.
 */
export type MixedControl = {
  kind: 'mixed';
  key: string;
  label: string;
  controls: EditControl[];
  /**
   * Makes the control's thumbnail, in sections that show one. The controls inside are not given thumbnails of their own, since the
   * row shows this control as one. Works as a slider's `thumb` does.
   */
  thumb?(photo: ImageLike, context: ToolContext): ImageLike | void;
  /**
   * The effect of the sliders inside, written against the Abra library as a slider's `tool` is, and made once for all of them.
   * It gets the values of each slider, in the order the sliders are found (including those of mixed controls inside this one),
   * with the values pickers saved on a slider after that slider's own. A slider with a `tool` of its own keeps it.
   */
  tool?(photo: ImageLike, values: number[][], context: ToolContext): ImageLike | void;
};

/**
 * A picture the user taps to choose an item: either a file shipped with the app, or the thumbnail the library renders for the
 * item's key (the same way effect thumbnails are drawn).
 */
export type PickerPicture = { kind: 'asset'; source: ImageSourcePropType } | { kind: 'rendered' };

export type PickerItem = {
  key: string;
  label: string;
  picture: PickerPicture;
  /** What tapping the item does, besides committing its `value` when the picker has a `commitTo`. */
  onTap?: () => void;
  /** The number saved when the item is chosen, for a picker with a `commitTo`. */
  value?: number;
  /**
   * Makes the item's picture from the photo, for an item whose `picture` is `rendered`: the editor draws it at thumbnail size, the
   * way it draws a one-tap control's thumbnail, and shows it in the row. It gets a thumbnail-size copy of the photo, which is its
   * own to change: it changes it in place and returns nothing, or returns a new image to use instead. An item without one shows
   * its label.
   */
  thumb?(photo: ImageLike, context: ToolContext): ImageLike | void;
};

/**
 * A control that opens a row of pictures, like the crop tool's aspect choices. Tapping an item calls its own `onTap`. With
 * `selection: 'one'` the last item tapped stays marked; with `'many'` each tapped item toggles its mark.
 *
 * With `commitTo`, tapping an item saves its `value` as the value at `index` of the slider control with that key, after the
 * values the sliders already hold, and the marked item is the one whose value is saved.
 */
export type PickerControl = {
  kind: 'picker';
  key: string;
  label: string;
  selection: 'one' | 'many';
  items: PickerItem[];
  commitTo?: { slider: string; index: number };
  /**
   * In a mixed control with a tabulated slider, the index of the slider tab this picker's tab comes after, or -1 for before the first
   * slider tab, where the control also opens. Without it, it comes last.
   */
  after?: number;
};

export type EditControl = SliderControl | ActionControl | ToolControl | GroupControl | MixedControl | PickerControl;

/**
 * Applies an action to `image`: its own `apply`, or else its tool.
 * The `apply` path is deprecated (see `ActionControl.apply`); use `tool`.
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
    release(photo);
    if (returned) release(returned);
  }
};

/** How a tabulated slider's pill shows its value (see `SliderControl.pillFormat`). */
export type PillFormat = 'signed' | 'percent' | 'plain';

/**
 * One slider of a control, with every setting filled in. A range slider is two of these in a row: the `low` and then the `high`
 * thumb, each holding one place of the control's value.
 */
export type SliderSpec = {
  label: string;
  min: number;
  max: number;
  step: number;
  defaultValue: number;
  valueText?: (value: number) => string | undefined;
  /** The place of the slider in the control's arrays (`sliderLabel`, `pillFormat`, `picker`), which a range slider shares. */
  entry: number;
  /** Which thumb of a range slider this is; not set for a plain slider. */
  range?: 'low' | 'high';
};

/** Whether the control has several sliders, which is when any of its per-slider settings is an array. */
export const isMultiSlider = (control: SliderControl): boolean =>
  [control.sliderLabel, control.min, control.max, control.step, control.defaultValue].some(Array.isArray);

/**
 * The sliders of a control, one for each item of its arrays, or the one slider of a control that has none. A range slider gives
 * two. Throws when the arrays are not all the same length.
 */
export const sliderSpecs = (control: SliderControl): SliderSpec[] => {
  const settings = [control.sliderLabel, control.min, control.max, control.step, control.defaultValue, control.pillFormat];
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
  // One thumb's share of a setting that may be a pair.
  const part = (setting: RangeSetting, thumb: 0 | 1) => (Array.isArray(setting) ? setting[thumb] : setting);
  return Array.from({ length: count }, (_, index): SliderSpec[] => {
    const base = {
      label: at(control.sliderLabel, index, control.label),
      valueText: at(control.valueText, index, undefined),
      entry: index,
    };
    const min = at<RangeSetting>(control.min, index, 0);
    const max = at<RangeSetting>(control.max, index, 100);
    const step = at<RangeSetting>(control.step, index, 1);
    const defaultValue = at<RangeSetting>(control.defaultValue, index, 0);
    const thumb = (which: 0 | 1, range?: 'low' | 'high'): SliderSpec => ({
      ...base,
      min: part(min, which),
      max: part(max, which),
      step: part(step, which),
      defaultValue: part(defaultValue, which),
      range,
    });
    return Array.isArray(defaultValue) ? [thumb(0, 'low'), thumb(1, 'high')] : [thumb(0)];
  }).flat();
};

/** What a control's value is while it has not been changed: a number for one slider, one number for each of several. */
export const sliderDefault = (control: SliderControl): AdjustmentValue => {
  const defaults = sliderSpecs(control).map(spec => spec.defaultValue);
  return isMultiSlider(control) ? defaults : defaults[0];
};

/**
 * The value of each slider, taken from the committed `value` and from the default where there is none. Values past the last
 * slider are kept as they are: they belong to something other than a slider, such as a picker that commits to this control.
 */
export const sliderValues = (control: SliderControl, value?: AdjustmentValue): number[] => {
  const defaults = sliderSpecs(control).map(spec => spec.defaultValue);
  const committed = value === undefined ? [] : Array.isArray(value) ? value : [value];
  return [...defaults.map((fallback, index) => committed[index] ?? fallback), ...committed.slice(defaults.length)];
};

/** The pill format of one slider of a control. */
export const pillFormatAt = (control: SliderControl, index: number): PillFormat =>
  (Array.isArray(control.pillFormat) ? control.pillFormat[index] : control.pillFormat) ?? 'signed';

/** The pill text of a range slider: its label and the low and high values, as `Orbs · 2–8`. */
export const rangePillText = (low: SliderSpec, lowValue: number, high: SliderSpec, highValue: number): string => {
  const show = (spec: SliderSpec, value: number) => {
    const decimals = spec.step >= 1 ? 0 : (String(spec.step).split('.')[1] ?? '').length;
    return spec.valueText?.(value) ?? value.toFixed(decimals);
  };
  return `${low.label} · ${show(low, lowValue)}–${show(high, highValue)}`;
};

/** The text of a tabulated slider's pill: its label and its value in `format`, rounded to the places its step has. */
export const pillText = (spec: SliderSpec, value: number, format: PillFormat): string => {
  const words = spec.valueText?.(value);
  if (words !== undefined) return format === 'plain' ? `${spec.label} · ${words}` : `${spec.label} (${words})`;
  const decimals = spec.step >= 1 ? 0 : (String(spec.step).split('.')[1] ?? '').length;
  const rounded = Number(value.toFixed(decimals));
  if (format === 'percent') {
    const span = spec.max - spec.min;
    const percent = span > 0 ? Math.round(((value - spec.min) / span) * 100) : 0;
    return `${spec.label} ${percent}%`;
  }
  if (format === 'plain') return `${spec.label} · ${rounded.toFixed(decimals)}`;
  const sign = rounded > 0 ? '+' : rounded < 0 ? '-' : '';
  return `${spec.label} (${sign}${Math.abs(rounded).toFixed(decimals)})`;
};

/** What a slider's tool is told besides the photo, when it is applied to `image` (the photo, or a copy of it at the same size). */
export const sliderContext = (
  control: SliderControl,
  image: AbraImageLike,
  scale: number,
  person?: ToolContext['person'],
  others?: number[],
  areas?: ToolContext['areas'],
): ToolContext => ({
  scale,
  preloaded: control.preload ? preloadFor(control, image, image) : undefined,
  person,
  others,
  areas,
  skin: () => skinMaskFor(person?.id, image, others),
});

/** Separates the rings of an area in its flat list. Points are clamped to 0 to 1, so this pair cannot be a real point. */
export const AREA_RING_BREAK = -1;

/** The rings of an area saved as a flat list of x and y fractions, or undefined when there is no area. Rings with no area are dropped. */
export const areaPoints = (value: AdjustmentValue | undefined): number[][][] | undefined => {
  if (!Array.isArray(value) || value.length < 6) return undefined;
  const rings: number[][][] = [[]];
  for (let index = 0; index + 1 < value.length; index += 2) {
    const x = value[index];
    const y = value[index + 1];
    if (x === AREA_RING_BREAK && y === AREA_RING_BREAK) rings.push([]);
    else rings[rings.length - 1].push([x, y]);
  }
  return rings.filter(ring => ring.length >= 3 && ringArea(ring) !== 0);
};

/** The flat list saved for a set of rings. Empty rings give a degenerate triangle, so an area that was fully taken away is not read as the whole photo. */
export const encodeAreaRings = (rings: number[][][]): number[] => {
  const flat = rings
    .filter(ring => ring.length >= 3)
    .flatMap((ring, index) => [
      ...(index > 0 ? [AREA_RING_BREAK, AREA_RING_BREAK] : []),
      ...ring.flatMap(([x, y]) => [Math.min(1, Math.max(0, x)), Math.min(1, Math.max(0, y))]),
    ]);
  return flat.length > 0 ? flat : [0, 0, 0, 0, 0, 0];
};

/** The shoelace area of a ring of `[x, y]` points: positive when it winds counter-clockwise, negative when clockwise. */
export const ringArea = (ring: number[][]): number =>
  ring.reduce((sum, [x, y], index) => {
    const [nextX, nextY] = ring[(index + 1) % ring.length];
    return sum + x * nextY - nextX * y;
  }, 0) / 2;

/** The sheet a control with a `layer` paints, for these slider values. `name` is the layer's name: the control's key, or its key for one person. */
export const layerSheet = (
  control: SliderControl,
  values: number[],
  context: ToolContext,
  name = control.key,
): Sheet => {
  const { layer } = control;
  if (!layer) throw new Error(`The control "${control.key}" has no layer to paint`);
  return {
    name,
    group: layer.group,
    overEarlier: layer.overEarlier,
    blend: typeof layer.blend === 'function' ? layer.blend(values) : layer.blend,
    opacity: layer.opacity(values),
    paint: sheet => layer.paint(sheet, values, context),
  };
};

/**
 * Applies a slider at `value` to `image`: its own `apply`, or else its tool.
 * The `apply` path is deprecated (see `SliderControl.apply`); use `tool`.
 */
export const applySlider = (
  control: SliderControl,
  image: AbraImageLike,
  value: AdjustmentValue,
  scale = 1,
  person?: ToolContext['person'],
  areas?: ToolContext['areas'],
) => {
  const values = sliderValues(control, value);
  const context = sliderContext(control, image, scale, person, undefined, areas);
  if (control.layer) {
    const laid = withLayers(image, [layerSheet(control, values, context)]);
    image.replaceWith(laid);
    release(laid);
  } else if (control.apply) control.apply(image, values[0], context);
  else if (control.tool) {
    const photo = image.clone();
    // The tool changed `photo` in place, or made a new image to use instead.
    const returned = control.tool(photo, values, context) as ImageLike | undefined;
    image.replaceWith(returned ?? photo);
    // They are native images the garbage collector cannot see the size of.
    release(photo);
    if (returned) release(returned);
  }
};

/** Applies the tool of a mixed control, made from several sliders, to `image`, given the values of each slider. */
export const applyJoint = (
  joint: NonNullable<SliderControl['joint']>,
  image: AbraImageLike,
  values: number[][],
  scale = 1,
) => {
  const photo = image.clone();
  const returned = joint.tool(photo, values, sliderContext(joint.sliders[0], image, scale)) as ImageLike | undefined;
  image.replaceWith(returned ?? photo);
  release(photo);
  if (returned) release(returned);
};

/** Whether a slider's effect is on, given its committed value (undefined when it has none). */
export const appliesOnSelect = (control: SliderControl) => control.applyOnSelect !== false;

export const isSliderApplied = (control: SliderControl, value: AdjustmentValue | undefined) =>
  appliesOnSelect(control)
    ? value !== undefined
    : sliderValues(control, value).some(
        (slider, index) => index < sliderSpecs(control).length && slider !== sliderSpecs(control)[index].defaultValue,
      );

export type EditSection = {
  key: string;
  label: string;
  controls: EditControl[];
  /** Show each control as a thumbnail of the effect applied, instead of a plain text chip. */
  previewThumbnails?: boolean;
};

/** Key of the blemish tool control; the editor dispatches on it to render the reticle and controls. */
export { BLEMISH_TOOL_KEY, CROP_TOOL_KEY, RED_EYE_TOOL_KEY, SKIN_SMOOTH_KEY, SKIN_TAN_KEY, SKIN_TONE_KEY };

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
  specialEffects,
  blur,
  detail,
  cleanup,
];

/**
 * The controls that do something, with every group and mixed control opened out into the controls inside it. A mixed control's
 * `tool` is given to each slider inside that has none of its own.
 */
export const leafControls = (controls: EditControl[]): Exclude<EditControl, GroupControl | MixedControl>[] =>
  controls.flatMap(control => {
    if (control.kind === 'group') return leafControls(control.controls);
    if (control.kind !== 'mixed') return [control];
    const { tool } = control;
    const inside = leafControls(control.controls);
    const joint = tool
      ? { key: control.key, tool, sliders: inside.filter((inner): inner is SliderControl => inner.kind === 'slider') }
      : undefined;
    return inside.map(inner =>
      joint && inner.kind === 'slider' && !inner.tool && !inner.joint ? { ...inner, joint } : inner,
    );
  });

/**
 * The controls that get a thumbnail: the leaf controls, except that a mixed control with a `thumb` counts as one, in place of
 * the controls inside it.
 */
export const thumbnailControls = (controls: EditControl[]): Exclude<EditControl, GroupControl>[] =>
  controls.flatMap(control =>
    control.kind === 'group' || (control.kind === 'mixed' && !control.thumb)
      ? thumbnailControls(control.controls)
      : [control],
  );

const holds = (control: EditControl, key: string): boolean =>
  control.key === key || ('controls' in control && control.controls.some(inner => holds(inner, key)));

/**
 * The sliders that are worked together with the control with this key: all the sliders of the outermost mixed control it sits in (the
 * tabs of Pop Art), or just itself. A group only holds controls side by side, so it is not counted. Empty when there is no such control.
 */
export const controlsWith = (controls: EditControl[], key: string): SliderControl[] => {
  for (const control of controls) {
    if (!holds(control, key)) continue;
    if (control.kind === 'group') return controlsWith(control.controls, key);
    return leafControls([control]).filter((inner): inner is SliderControl => inner.kind === 'slider');
  }
  return [];
};

/** The mixed control that holds the control with this key directly, if there is one. */
export const mixedParent = (controls: EditControl[], key: string): MixedControl | undefined => {
  for (const control of controls) {
    if (control.kind !== 'group' && control.kind !== 'mixed') continue;
    if (control.kind === 'mixed' && control.controls.some(inner => inner.key === key)) return control;
    const inside = mixedParent(control.controls, key);
    if (inside) return inside;
  }
  return undefined;
};

/**
 * The groups and tables to open, outermost first, to reach the one with this key at any depth: the last is that one and the one
 * before it is the one to go back to. Empty when there is no such one.
 */
export const groupPath = (controls: EditControl[], key: string): (GroupControl | MixedControl | PickerControl)[] => {
  for (const control of controls) {
    if (control.kind === 'picker') {
      if (control.key === key) return [control];
      continue;
    }
    if (control.kind !== 'group' && control.kind !== 'mixed') continue;
    if (control.key === key) return [control];
    const inside = groupPath(control.controls, key);
    if (inside.length) return [control, ...inside];
  }
  return [];
};

export const findSection = (key: string): EditSection =>
  EDIT_SECTIONS.find(section => section.key === key) ?? EDIT_SECTIONS[0];

// What each slider says about itself (`person`, `face`) is noted for the state, which reads it without the controls.
for (const control of EDIT_SECTIONS.flatMap(section => leafControls(section.controls))) {
  if (control.kind === 'slider') setControlTraits(control.key, control);
}
