import type { PickedImage } from '@/src/components/file-picker';
import type { ColorSwatch } from '@/src/state/color-picker';
import type { CurvePreset, CurveValue } from '@/src/state/curve-picker';
import type { GradientValue } from '@/src/state/gradient-picker';
import { isReal, realBetween, realOr, warnNotReal } from '@/src/lib/plugins/plugin-values';
import { BehaviorSubject } from 'rxjs';

/** Where a file picker looks: the device's files, the app's saved projects (their edited images), or both. */
export type PluginFilePickerSource = 'file-system' | 'projects' | 'both';

export type PluginFilePickerOptions = {
  /** Where the user can pick from. Defaults to `'file-system'`. */
  source?: PluginFilePickerSource;
  multiple?: boolean;
  /** Show a thumbnail of each picked image below the control. Defaults to false. */
  thumbs?: boolean;
  onPick: (files: PickedImage[]) => void;
};

export type PluginSliderOptions = {
  /** The lowest value. Defaults to 0. */
  min?: number;
  max: number;
  /** How far one notch moves the value. Defaults to 1. */
  step?: number;
  /** Shows a reset button beside the name, which puts the slider back to where it was when the editor opened. Defaults to true. */
  reset?: boolean;
  /** Puts the name on a line of its own with the slider under it, across the whole width. Defaults to false for a slider with one thumb, where the name is on the left and the slider on the right, and to true for a range slider, which needs the width. */
  multiline?: boolean;
  /** Runs the plugin again once the slider stops moving (the preview is drawn on every move meanwhile), for a value that changes how the picture is made (not only where a finished piece sits). Defaults to false. */
  rebuild?: boolean;
} & (
  | {
      /** One thumb. This is the default. */
      range?: false;
      /** Where the thumb starts. Defaults to `min`. */
      value?: number;
      onChange?: (value: number) => void;
    }
  | {
      /** Two thumbs, so the value is a low and a high. */
      range: true;
      /** Where the thumbs start. Defaults to `[min, max]`. */
      value?: [number, number];
      onChange?: (value: [number, number]) => void;
    }
);

/** Where a control is shown: the inspector beside the preview, or the controls bar under it. The host sets it from the export that made the control. */
export type PluginSurface = 'inspector' | 'controls';

export type PluginSizeOptions = {
  /** What the two numbers start as. Defaults to `[0, 0]`. */
  value?: [number, number];
  /** What each field is called, above it. Defaults to `['Width', 'Height']`. */
  labels?: [string, string];
  /** Shows a reset button beside the name, which puts the numbers back to what they were when the editor opened. Defaults to true. */
  reset?: boolean;
  /** The lowest either number can be. A smaller number is raised to it. No limit when left out. */
  min?: number;
  /** The highest either number can be. A larger number is lowered to it. No limit when left out. */
  max?: number;
  /** Called with both numbers each time one of them is changed to a number. */
  onChange?: (value: [number, number]) => void;
};

export type PluginSelectOptions = {
  /** The choices, in the order they are shown, as a bar of buttons. */
  options: string[];
  /** The choice that starts selected. Defaults to the first. */
  value?: string;
  /** Makes the bar a row of buttons that act when tapped and do not stay selected (previous, next, random). Defaults to false. */
  momentary?: boolean;
  /** Called with the choice each time one is picked. The editor runs the plugin again after it, since a choice changes what it makes. */
  onChange?: (value: string) => void;
};

export type PluginNumberOptions = {
  /** What the number starts as. Defaults to `min`, or 0 when there is no `min`. */
  value?: number;
  /** The lowest the number can be. A smaller number is raised to it. No limit when left out. */
  min?: number;
  /** The highest the number can be. A larger number is lowered to it. No limit when left out. */
  max?: number;
  /** How far one step is, which is also how many decimals it shows. Defaults to 1. */
  step?: number;
  /** Shows a reset button beside the name, which puts the number back to what it was when the editor opened. Defaults to true. */
  reset?: boolean;
  /** Called with the number each time what is typed is one. The editor runs the plugin again after it. */
  onChange?: (value: number) => void;
};

export type PluginTextOptions = {
  /** What the box starts with. Defaults to nothing. */
  value?: string;
  /** Shown in the box while it is empty. */
  placeholder?: string;
  /** Called with the text a moment after typing stops (not on every letter). The editor runs the plugin again after it. */
  onChange?: (value: string) => void;
};

export type PluginFontPickerOptions = {
  /** The family that starts chosen. Shows "Choose a font" when it is left out. */
  value?: string;
  /** Called with the family each time one is picked on the Fonts overlay. The editor runs the plugin again after it. */
  onPick?: (family: string) => void;
};

export type PluginPicturesOptions = {
  /** The pictures to show, in order. */
  files: PickedImage[];
  /** How many rows of thumbnails there are, with the rest reached by scrolling sideways, or `'all'` to show every picture. Defaults to 2. */
  rows?: number | 'all';
  /** Called with the place in `files` of the picture that was tapped. The editor runs the plugin again after it. */
  onPick?: (index: number) => void;
};

export type PluginColorOptions = {
  /** The color it starts as: `[red, green, blue]`, each 0 to 255, with the strength as a fourth number (0 to 255) when `alpha` is true. Defaults to white. */
  value?: number[];
  /** Whether the color has a strength, so the picker has a Strength bar and the value has a fourth number. Defaults to false. */
  alpha?: boolean;
  /** The words over the picker. Defaults to the control's name. */
  title?: string;
  /** The named colors the picker shows above the square, each `{ label, rgb: [r, g, b] }`. Defaults to the app's common colors. */
  swatches?: ColorSwatch[];
  /** Called with `[red, green, blue]` (and the strength when `alpha` is true) each time a color is picked and OK is pressed. The editor runs the plugin again after it. */
  onChange?: (value: number[]) => void;
  /**
   * Called as the person drags in the picker, with the color it shows now, so the plugin can show the change before OK is pressed. When the picker is
   * cancelled it is called once more with the color the control had, so the change is taken back. The editor does not run the plugin again for it:
   * change what is showing, as a slider's `onChange` does.
   */
  onPreview?: (value: number[]) => void;
};

export type PluginGradientOptions = {
  /**
   * The gradient it starts as: `colors`, each `{ at, color: [red, green, blue] }`, and `alphas`, each `{ at, alpha }`, with `at` from 0 (the start)
   * to 1 (the end) and `alpha` from 0 (not there) to 255 (solid). The strength has stops of its own, so where it changes need not be where the
   * color does. Each list needs a stop; one that has none gets a white one (or a solid one). Defaults to solid white.
   */
  value?: GradientValue;
  /** The words over the editor. Defaults to the control's name. */
  title?: string;
  /** The named colors the color picker of a stop shows, each `{ label, rgb: [r, g, b] }`. Defaults to the app's common colors. */
  swatches?: ColorSwatch[];
  /** Called with the gradient (each list in order along it) each time the editor is closed with OK. The editor runs the plugin again after it. */
  onChange?: (value: GradientValue) => void;
  /**
   * Called as the person edits in the editor, with the gradient it shows now, so the plugin can show the change before OK is pressed. When the editor
   * is cancelled it is called once more with the gradient the control had, so the change is taken back. The editor does not run the plugin again for it.
   */
  onPreview?: (value: GradientValue) => void;
};

export type PluginCurveOptions = {
  /** The points it starts as, each `[x, y]`: `x` from 0 (the start) to 1 (the end) and `y` from `min` to `max`. It is a smooth line through them, flat before the first and after the last. Needs a point; with none it is flat at `max`. */
  value?: CurveValue;
  /** The lowest and highest value of the curve, which is also the range of the editor. Default to 0 and 1. */
  min?: number;
  max?: number;
  /** The words over the editor. Defaults to the control's name. */
  title?: string;
  /** Named shapes the editor offers to start from, each `{ label, value }` with `value` as above. */
  presets?: CurvePreset[];
  /** Called with the points (in order along the curve) each time the editor is closed with OK. The editor runs the plugin again after it. */
  onChange?: (value: CurveValue) => void;
  /**
   * Called as the person edits in the editor, with the curve it shows now, so the plugin can show the change before OK is pressed. When the editor
   * is cancelled it is called once more with the curve the control had, so the change is taken back. The editor does not run the plugin again for it.
   */
  onPreview?: (value: CurveValue) => void;
};

export type PluginToggleOptions = {
  /** Whether the toggle starts on. Defaults to false. */
  value?: boolean;
  /** Called with whether it is on each time it is turned. The editor runs the plugin again after it. */
  onChange?: (value: boolean) => void;
};

/** Which heading it is, like <h1> to <h6>. */
export type PluginHeaderLevel = 1 | 2 | 3 | 4 | 5 | 6;

/** `key` names the control among the others of its plugin, so two with the same label stay apart. The host makes it unique each time the plugin's inspector is made. */
type PluginInspectorBase = { id: number; key: string; surface: PluginSurface; pluginId: string; label: string };

export type PluginInspectorItem =
  | (PluginInspectorBase & {
      kind: 'file-picker';
      source: PluginFilePickerSource;
      multiple: boolean;
      thumbs: boolean;
      onPick: (files: PickedImage[]) => void;
    })
  | (PluginInspectorBase & { kind: 'header'; level: PluginHeaderLevel; back?: () => void })
  | (PluginInspectorBase & { kind: 'group'; open: boolean; enabled?: boolean; onToggle?: (enabled: boolean) => void })
  | (PluginInspectorBase & { kind: 'submenu'; onOpen?: () => void })
  | (PluginInspectorBase & { kind: 'select'; options: string[]; value: string; momentary?: boolean; onChange?: (value: string) => void })
  | (PluginInspectorBase & { kind: 'font-picker'; value?: string; onPick?: (family: string) => void })
  | (PluginInspectorBase & { kind: 'pictures'; files: PickedImage[]; rows: number | 'all'; onPick?: (index: number) => void })
  | (PluginInspectorBase & { kind: 'toggle'; value: boolean; onChange?: (value: boolean) => void })
  | (PluginInspectorBase & {
      kind: 'color';
      value: number[];
      alpha: boolean;
      title?: string;
      swatches?: ColorSwatch[];
      onChange?: (value: number[]) => void;
      onPreview?: (value: number[]) => void;
    })
  | (PluginInspectorBase & {
      kind: 'gradient';
      value: GradientValue;
      title?: string;
      swatches?: ColorSwatch[];
      onChange?: (value: GradientValue) => void;
      onPreview?: (value: GradientValue) => void;
    })
  | (PluginInspectorBase & {
      kind: 'curve';
      value: CurveValue;
      min: number;
      max: number;
      title?: string;
      presets?: CurvePreset[];
      onChange?: (value: CurveValue) => void;
      onPreview?: (value: CurveValue) => void;
    })
  | (PluginInspectorBase & { kind: 'text'; value: string; placeholder?: string; onChange?: (value: string) => void })
  | (PluginInspectorBase & {
      kind: 'number';
      value: number;
      min?: number;
      max?: number;
      step: number;
      reset: boolean;
      onChange?: (value: number) => void;
    })
  | (PluginInspectorBase & {
      kind: 'size';
      value: [number, number];
      labels: [string, string];
      reset: boolean;
      min?: number;
      max?: number;
      onChange?: (value: [number, number]) => void;
    })
  | (PluginInspectorBase & {
      kind: 'slider';
      min: number;
      max: number;
      step: number;
      range: boolean;
      multiline: boolean;
      reset: boolean;
      rebuild: boolean;
      /** Where the thumb or thumbs start. */
      value: number | [number, number];
      onChange?: (value: number | [number, number]) => void;
    });

let nextId = 0;
export const pluginInspectorItems$ = new BehaviorSubject<PluginInspectorItem[]>([]);
const activeLayouts = new Map<string, PluginInspectorItem[]>();

export type PluginHeaderOptions = {
  /** Which heading it is. Defaults to 1. */
  level?: PluginHeaderLevel;
  /** Shows a Back button with the heading, which calls this when it is tapped. For the top of a sub-inspector, to return to the main one. */
  back?: () => void;
};

export function createPluginHeaderItem(pluginId: string, text: string, options: PluginHeaderOptions = {}): PluginInspectorItem {
  return { id: nextId++, key: text, surface: 'inspector', pluginId, kind: 'header', label: text, level: options.level ?? 1, back: options.back };
}

export function addPluginHeader(pluginId: string, text: string, options?: PluginHeaderOptions): void {
  const item = createPluginHeaderItem(pluginId, text, options);
  const layout = activeLayouts.get(pluginId);
  if (layout) {
    layout.push(item);
  } else {
    pluginInspectorItems$.next([...pluginInspectorItems$.value, item]);
  }
}

export type PluginGroupOptions = {
  /** Whether the group starts open. Defaults to false. The person's own taps on it win over this once they have made one. */
  open?: boolean;
  /** Puts a checkbox in the group's title, ticked when this is true, for a group that can be switched on and off as a whole (the group still folds with its arrow). Left out, the group has no checkbox. */
  enabled?: boolean;
  /** Called with whether the group is switched on each time its checkbox is tapped. The editor runs the plugin again after it. */
  onToggle?: (enabled: boolean) => void;
};

export function createPluginGroupItem(pluginId: string, label: string, options: PluginGroupOptions = {}): PluginInspectorItem {
  return { id: nextId++, key: label, surface: 'inspector', pluginId, kind: 'group', label, open: options.open ?? false, enabled: options.enabled, onToggle: options.onToggle };
}

export function addPluginGroup(pluginId: string, label: string, options?: PluginGroupOptions): void {
  const item = createPluginGroupItem(pluginId, label, options);
  const layout = activeLayouts.get(pluginId);
  if (layout) {
    layout.push(item);
  } else {
    pluginInspectorItems$.next([...pluginInspectorItems$.value, item]);
  }
}

export type PluginSubmenuOptions = {
  /** Called when the row is tapped. The editor runs the plugin again after it, so it can make the submenu's controls (with a `header` that has `back`) in place of the main ones. */
  onOpen?: () => void;
};

export function createPluginSubmenuItem(pluginId: string, label: string, options: PluginSubmenuOptions = {}): PluginInspectorItem {
  return { id: nextId++, key: label, surface: 'inspector', pluginId, kind: 'submenu', label, onOpen: options.onOpen };
}

export function addPluginSubmenu(pluginId: string, label: string, options?: PluginSubmenuOptions): void {
  const item = createPluginSubmenuItem(pluginId, label, options);
  const layout = activeLayouts.get(pluginId);
  if (layout) {
    layout.push(item);
  } else {
    pluginInspectorItems$.next([...pluginInspectorItems$.value, item]);
  }
}

/** The number when it is real, and when it is not, `fallback`, with the plugin told once that it gave a bad one. */
const realValue = (pluginId: string, label: string, value: unknown, fallback: number) => {
  if (isReal(value)) return value;
  warnNotReal(pluginId, `the control "${label}"`, fallback);
  return fallback;
};

/** Two numbers made real. */
const realPair = (pluginId: string, label: string, value: [number, number], fallback: [number, number]): [number, number] => [
  realValue(pluginId, label, value[0], fallback[0]),
  realValue(pluginId, label, value[1], fallback[1]),
];

/** A limit that is a real number, or nothing, so a limit that is not real is no limit and not a NaN that every comparison fails. */
const realOrUndefined = (value: number | undefined) => (isReal(value) ? value : undefined);

/** A step that is more than nothing, or 1. */
const positiveStep = (step: number | undefined) => (isReal(step) && step > 0 ? step : 1);

export function createPluginSizeItem(pluginId: string, label: string, options: PluginSizeOptions = {}): PluginInspectorItem {
  return {
    id: nextId++,
    key: label,
    surface: 'inspector',
    pluginId,
    kind: 'size',
    label,
    value: realPair(pluginId, label, options.value ?? [0, 0], [0, 0]),
    labels: options.labels ?? ['Width', 'Height'],
    reset: options.reset ?? true,
    min: realOrUndefined(options.min),
    max: realOrUndefined(options.max),
    onChange: options.onChange,
  };
}

export function addPluginSize(pluginId: string, label: string, options?: PluginSizeOptions): void {
  const item = createPluginSizeItem(pluginId, label, options);
  const layout = activeLayouts.get(pluginId);
  if (layout) {
    layout.push(item);
  } else {
    pluginInspectorItems$.next([...pluginInspectorItems$.value, item]);
  }
}

export function createPluginSelectItem(pluginId: string, label: string, options: PluginSelectOptions): PluginInspectorItem {
  return {
    id: nextId++,
    key: label,
    surface: 'inspector',
    pluginId,
    kind: 'select',
    label,
    options: options.options,
    value: options.value ?? options.options[0] ?? '',
    momentary: options.momentary,
    onChange: options.onChange,
  };
}

export function addPluginSelect(pluginId: string, label: string, options : PluginSelectOptions): void {
  const item = createPluginSelectItem(pluginId, label, options);
  const layout = activeLayouts.get(pluginId);
  if (layout) {
    layout.push(item);
  } else {
    pluginInspectorItems$.next([...pluginInspectorItems$.value, item]);
  }
}

export function createPluginTextItem(pluginId: string, label: string, options: PluginTextOptions = {}): PluginInspectorItem {
  return { id: nextId++, key: label, surface: 'inspector', pluginId, kind: 'text', label, value: options.value ?? '', placeholder: options.placeholder, onChange: options.onChange };
}

export function addPluginText(pluginId: string, label: string, options?: PluginTextOptions): void {
  const item = createPluginTextItem(pluginId, label, options);
  const layout = activeLayouts.get(pluginId);
  if (layout) {
    layout.push(item);
  } else {
    pluginInspectorItems$.next([...pluginInspectorItems$.value, item]);
  }
}

export function createPluginFontPickerItem(pluginId: string, label: string, options: PluginFontPickerOptions = {}): PluginInspectorItem {
  return { id: nextId++, key: label, surface: 'inspector', pluginId, kind: 'font-picker', label, value: options.value, onPick: options.onPick };
}

export function addPluginFontPicker(pluginId: string, label: string, options?: PluginFontPickerOptions): void {
  const item = createPluginFontPickerItem(pluginId, label, options);
  const layout = activeLayouts.get(pluginId);
  if (layout) {
    layout.push(item);
  } else {
    pluginInspectorItems$.next([...pluginInspectorItems$.value, item]);
  }
}

export function createPluginPicturesItem(pluginId: string, label: string, options: PluginPicturesOptions): PluginInspectorItem {
  return { id: nextId++, key: label, surface: 'inspector', pluginId, kind: 'pictures', label, files: options.files, rows: options.rows ?? 2, onPick: options.onPick };
}

export function addPluginPictures(pluginId: string, label: string, options: PluginPicturesOptions): void {
  const item = createPluginPicturesItem(pluginId, label, options);
  const layout = activeLayouts.get(pluginId);
  if (layout) {
    layout.push(item);
  } else {
    pluginInspectorItems$.next([...pluginInspectorItems$.value, item]);
  }
}

export function createPluginToggleItem(pluginId: string, label: string, options: PluginToggleOptions = {}): PluginInspectorItem {
  return { id: nextId++, key: label, surface: 'inspector', pluginId, kind: 'toggle', label, value: options.value ?? false, onChange: options.onChange };
}

export function addPluginToggle(pluginId: string, label: string, options?: PluginToggleOptions): void {
  const item = createPluginToggleItem(pluginId, label, options);
  const layout = activeLayouts.get(pluginId);
  if (layout) {
    layout.push(item);
  } else {
    pluginInspectorItems$.next([...pluginInspectorItems$.value, item]);
  }
}

export function createPluginColorItem(pluginId: string, label: string, options: PluginColorOptions = {}): PluginInspectorItem {
  const alpha = options.alpha ?? false;
  const given = options.value ?? (alpha ? [255, 255, 255, 255] : [255, 255, 255]);
  // Each channel is a whole number from 0 to 255; one that is not real is white (or solid, for the strength).
  const value = [0, 1, 2, 3].map(at => Math.round(realBetween(realValue(pluginId, label, given[at] ?? 255, 255), 0, 255, 255)));
  return {
    id: nextId++,
    key: label,
    surface: 'inspector',
    pluginId,
    kind: 'color',
    label,
    value: alpha ? [value[0], value[1], value[2], value[3] ?? 255] : value.slice(0, 3),
    alpha,
    title: options.title,
    swatches: options.swatches,
    onChange: options.onChange,
    onPreview: options.onPreview,
  };
}

export function addPluginColor(pluginId: string, label: string, options?: PluginColorOptions): void {
  const item = createPluginColorItem(pluginId, label, options);
  const layout = activeLayouts.get(pluginId);
  if (layout) {
    layout.push(item);
  } else {
    pluginInspectorItems$.next([...pluginInspectorItems$.value, item]);
  }
}

/** A gradient with every stop real and in range, each list sorted and not empty, whatever the plugin gave. */
const realGradient = (pluginId: string, label: string, value: GradientValue | undefined): GradientValue => {
  const place = (at: unknown) => realBetween(realValue(pluginId, label, at, 0), 0, 1, 0);
  const channel = (each: unknown) => Math.round(realBetween(realValue(pluginId, label, each, 255), 0, 255, 255));
  const colors = (value?.colors ?? [])
    .map(stop => ({ at: place(stop?.at), color: [0, 1, 2].map(index => channel(stop?.color?.[index])) as [number, number, number] }))
    .sort((a, b) => a.at - b.at);
  const alphas = (value?.alphas ?? []).map(stop => ({ at: place(stop?.at), alpha: channel(stop?.alpha) })).sort((a, b) => a.at - b.at);
  return {
    colors: colors.length ? colors : [{ at: 0, color: [255, 255, 255] }],
    alphas: alphas.length ? alphas : [{ at: 0, alpha: 255 }],
  };
};

export function createPluginGradientItem(pluginId: string, label: string, options: PluginGradientOptions = {}): PluginInspectorItem {
  return {
    id: nextId++,
    key: label,
    surface: 'inspector',
    pluginId,
    kind: 'gradient',
    label,
    value: realGradient(pluginId, label, options.value),
    title: options.title,
    swatches: options.swatches,
    onChange: options.onChange,
    onPreview: options.onPreview,
  };
}

export function addPluginGradient(pluginId: string, label: string, options?: PluginGradientOptions): void {
  const item = createPluginGradientItem(pluginId, label, options);
  const layout = activeLayouts.get(pluginId);
  if (layout) {
    layout.push(item);
  } else {
    pluginInspectorItems$.next([...pluginInspectorItems$.value, item]);
  }
}

export function createPluginCurveItem(pluginId: string, label: string, options: PluginCurveOptions = {}): PluginInspectorItem {
  const low = realOr(options.min ?? 0, 0);
  const given = realOr(options.max ?? 1, low + 1);
  const high = given > low ? given : low + 1;
  const held = (value: unknown, fallback: number, from: number, to: number) => realBetween(realValue(pluginId, label, value, fallback), from, to, fallback);
  const points = (options.value ?? [])
    .map(point => [held(point?.[0], 0, 0, 1), held(point?.[1], high, low, high)] as [number, number])
    .sort((a, b) => a[0] - b[0]);
  return {
    id: nextId++,
    key: label,
    surface: 'inspector',
    pluginId,
    kind: 'curve',
    label,
    value: points.length ? points : [[0, high], [1, high]],
    min: low,
    max: high,
    title: options.title,
    presets: options.presets,
    onChange: options.onChange,
    onPreview: options.onPreview,
  };
}

export function addPluginCurve(pluginId: string, label: string, options?: PluginCurveOptions): void {
  const item = createPluginCurveItem(pluginId, label, options);
  const layout = activeLayouts.get(pluginId);
  if (layout) {
    layout.push(item);
  } else {
    pluginInspectorItems$.next([...pluginInspectorItems$.value, item]);
  }
}

export function createPluginNumberItem(pluginId: string, label: string, options: PluginNumberOptions = {}): PluginInspectorItem {
  return {
    id: nextId++,
    key: label,
    surface: 'inspector',
    pluginId,
    kind: 'number',
    label,
    value: realValue(pluginId, label, options.value ?? options.min ?? 0, realOr(options.min, 0)),
    min: realOrUndefined(options.min),
    max: realOrUndefined(options.max),
    step: positiveStep(options.step),
    reset: options.reset ?? true,
    onChange: options.onChange,
  };
}

export function addPluginNumber(pluginId: string, label: string, options ?: PluginNumberOptions): void {
  const item = createPluginNumberItem(pluginId, label, options);
  const layout = activeLayouts.get(pluginId);
  if (layout) {
    layout.push(item);
  } else {
    pluginInspectorItems$.next([...pluginInspectorItems$.value, item]);
  }
}

export function createPluginSliderItem(pluginId: string, label: string, options: PluginSliderOptions): PluginInspectorItem {
  // A slider needs a lowest and a highest that are real and the right way round, or it cannot be drawn and its value means nothing.
  const min = realOr(options.min ?? 0, 0);
  const given = realOr(options.max, min + 1);
  const max = given > min ? given : min + 1;
  const held = (value: number) => realBetween(value, min, max, min);
  const start = options.value ?? (options.range ? [min, max] : min);
  const value: number | [number, number] = Array.isArray(start)
    ? [held(realValue(pluginId, label, start[0], min)), held(realValue(pluginId, label, start[1], max))]
    : held(realValue(pluginId, label, start, min));
  return {
    id: nextId++,
    key: label,
    surface: 'inspector',
    pluginId,
    kind: 'slider',
    label,
    min,
    max,
    step: positiveStep(options.step),
    range: options.range ?? false,
    multiline: options.multiline ?? options.range ?? false,
    reset: options.reset ?? true,
    rebuild: options.rebuild ?? false,
    value,
    onChange: options.onChange as ((value: number | [number, number]) => void) | undefined,
  };
}

export function addPluginSlider(pluginId: string, label: string, options: PluginSliderOptions): void {
  const item = createPluginSliderItem(pluginId, label, options);
  const layout = activeLayouts.get(pluginId);
  if (layout) {
    layout.push(item);
  } else {
    pluginInspectorItems$.next([...pluginInspectorItems$.value, item]);
  }
}

export function createPluginFilePickerItem(pluginId: string, label: string, options: PluginFilePickerOptions): PluginInspectorItem {
  return {
    id: nextId++,
    key: label,
    surface: 'inspector',
    pluginId,
    kind: 'file-picker',
    label,
    source: options.source ?? 'file-system',
    multiple: options.multiple ?? false,
    thumbs: options.thumbs ?? false,
    onPick: options.onPick,
  };
}

export function addPluginFilePicker(pluginId: string, label: string, options: PluginFilePickerOptions): void {
  const item = createPluginFilePickerItem(pluginId, label, options);
  const layout = activeLayouts.get(pluginId);
  if (layout) {
    layout.push(item);
  } else {
    pluginInspectorItems$.next([...pluginInspectorItems$.value, item]);
  }
}

/** Runs a plugin's inspectorGUI export and atomically replaces its prior inspector items. */
export function renderPluginInspector(pluginId: string, render: () => void): PluginInspectorItem[] {
  const previous = pluginInspectorItems$.value.filter(item => item.pluginId === pluginId);
  const next: PluginInspectorItem[] = [];
  activeLayouts.set(pluginId, next);
  try {
    render();
  } finally {
    activeLayouts.delete(pluginId);
  }

  const layout = next.map((item, index) => ({ ...item, id: previous[index]?.id ?? item.id }));
  const unchanged = previous.length === layout.length && previous.every((item, index) => {
    const replacement = layout[index];
    if (item.kind === 'file-picker' && replacement.kind === 'file-picker') {
      return item.label === replacement.label && item.source === replacement.source && item.multiple === replacement.multiple &&
        item.thumbs === replacement.thumbs && item.onPick === replacement.onPick;
    }
    if (item.kind === 'header' && replacement.kind === 'header') {
      return item.label === replacement.label && item.level === replacement.level && item.back === replacement.back;
    }
    if (item.kind === 'group' && replacement.kind === 'group') {
      return item.label === replacement.label && item.open === replacement.open && item.enabled === replacement.enabled &&
        item.onToggle === replacement.onToggle;
    }
    if (item.kind === 'submenu' && replacement.kind === 'submenu') {
      return item.label === replacement.label && item.onOpen === replacement.onOpen;
    }
    if (item.kind === 'select' && replacement.kind === 'select') {
      return item.label === replacement.label && item.value === replacement.value && item.onChange === replacement.onChange &&
        item.options.length === replacement.options.length && item.options.every((each, at) => each === replacement.options[at]);
    }
    if (item.kind === 'toggle' && replacement.kind === 'toggle') {
      return item.label === replacement.label && item.value === replacement.value && item.onChange === replacement.onChange;
    }
    if (item.kind === 'color' && replacement.kind === 'color') {
      return item.label === replacement.label && item.alpha === replacement.alpha && item.title === replacement.title &&
        item.swatches === replacement.swatches && item.onChange === replacement.onChange && item.onPreview === replacement.onPreview &&
        item.value.length === replacement.value.length && item.value.every((each, at) => each === replacement.value[at]);
    }
    if (item.kind === 'gradient' && replacement.kind === 'gradient') {
      return item.label === replacement.label && item.title === replacement.title && item.swatches === replacement.swatches &&
        item.onChange === replacement.onChange && item.onPreview === replacement.onPreview &&
        JSON.stringify(item.value) === JSON.stringify(replacement.value);
    }
    if (item.kind === 'curve' && replacement.kind === 'curve') {
      return item.label === replacement.label && item.min === replacement.min && item.max === replacement.max && item.title === replacement.title &&
        item.presets === replacement.presets && item.onChange === replacement.onChange && item.onPreview === replacement.onPreview &&
        JSON.stringify(item.value) === JSON.stringify(replacement.value);
    }
    if (item.kind === 'text' && replacement.kind === 'text') {
      return item.label === replacement.label && item.value === replacement.value && item.placeholder === replacement.placeholder &&
        item.onChange === replacement.onChange;
    }
    if (item.kind === 'pictures' && replacement.kind === 'pictures') {
      return item.label === replacement.label && item.files === replacement.files && item.rows === replacement.rows &&
        item.onPick === replacement.onPick;
    }
    if (item.kind === 'font-picker' && replacement.kind === 'font-picker') {
      return item.label === replacement.label && item.value === replacement.value && item.onPick === replacement.onPick;
    }
    if (item.kind === 'number' && replacement.kind === 'number') {
      return item.label === replacement.label && item.value === replacement.value && item.min === replacement.min &&
        item.max === replacement.max && item.step === replacement.step && item.reset === replacement.reset &&
        item.onChange === replacement.onChange;
    }
    if (item.kind === 'size' && replacement.kind === 'size') {
      return item.label === replacement.label && item.value[0] === replacement.value[0] && item.value[1] === replacement.value[1] &&
        item.labels[0] === replacement.labels[0] && item.labels[1] === replacement.labels[1] && item.reset === replacement.reset &&
        item.min === replacement.min && item.max === replacement.max && item.onChange === replacement.onChange;
    }
    if (item.kind === 'slider' && replacement.kind === 'slider') {
      return item.label === replacement.label && item.min === replacement.min && item.max === replacement.max &&
        item.step === replacement.step && item.range === replacement.range && item.multiline === replacement.multiline && item.reset === replacement.reset && item.rebuild === replacement.rebuild && item.onChange === replacement.onChange;
    }
    return false;
  });
  if (unchanged) return previous;

  pluginInspectorItems$.next([
    ...pluginInspectorItems$.value.filter(item => item.pluginId !== pluginId),
    ...layout,
  ]);
  return layout;
}

export function removePluginInspectorItem(id: number): void {
  pluginInspectorItems$.next(pluginInspectorItems$.value.filter(item => item.id !== id));
}

export function removePluginInspectorItems(pluginId: string): void {
  pluginInspectorItems$.next(pluginInspectorItems$.value.filter(item => item.pluginId !== pluginId));
}
