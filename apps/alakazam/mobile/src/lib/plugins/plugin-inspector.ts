import type { PickedImage } from '@/src/components/file-picker';
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
  | (PluginInspectorBase & { kind: 'header'; level: PluginHeaderLevel })
  | (PluginInspectorBase & { kind: 'select'; options: string[]; value: string; onChange?: (value: string) => void })
  | (PluginInspectorBase & { kind: 'toggle'; value: boolean; onChange?: (value: boolean) => void })
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
};

export function createPluginHeaderItem(pluginId: string, text: string, options: PluginHeaderOptions = {}): PluginInspectorItem {
  return { id: nextId++, key: text, surface: 'inspector', pluginId, kind: 'header', label: text, level: options.level ?? 1 };
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

export function createPluginSizeItem(pluginId: string, label: string, options: PluginSizeOptions = {}): PluginInspectorItem {
  return {
    id: nextId++,
    key: label,
    surface: 'inspector',
    pluginId,
    kind: 'size',
    label,
    value: options.value ?? [0, 0],
    labels: options.labels ?? ['Width', 'Height'],
    reset: options.reset ?? true,
    min: options.min,
    max: options.max,
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

export function createPluginNumberItem(pluginId: string, label: string, options: PluginNumberOptions = {}): PluginInspectorItem {
  return {
    id: nextId++,
    key: label,
    surface: 'inspector',
    pluginId,
    kind: 'number',
    label,
    value: options.value ?? options.min ?? 0,
    min: options.min,
    max: options.max,
    step: options.step ?? 1,
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
  return {
    id: nextId++,
    key: label,
    surface: 'inspector',
    pluginId,
    kind: 'slider',
    label,
    min: options.min ?? 0,
    max: options.max,
    step: options.step ?? 1,
    range: options.range ?? false,
    multiline: options.multiline ?? options.range ?? false,
    reset: options.reset ?? true,
    value: options.value ?? (options.range ? [options.min ?? 0, options.max] : options.min ?? 0),
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
      return item.label === replacement.label && item.level === replacement.level;
    }
    if (item.kind === 'select' && replacement.kind === 'select') {
      return item.label === replacement.label && item.value === replacement.value && item.onChange === replacement.onChange &&
        item.options.length === replacement.options.length && item.options.every((each, at) => each === replacement.options[at]);
    }
    if (item.kind === 'toggle' && replacement.kind === 'toggle') {
      return item.label === replacement.label && item.value === replacement.value && item.onChange === replacement.onChange;
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
        item.step === replacement.step && item.range === replacement.range && item.multiline === replacement.multiline && item.reset === replacement.reset && item.onChange === replacement.onChange;
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
