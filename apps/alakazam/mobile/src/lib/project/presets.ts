import { EDIT_SECTIONS, leafControls, type AdjustmentValue, type EditControl } from '@/src/lib/project/edit-sections';
import type { ProjectRecipe } from '@/src/lib/project/project-store';
import { NativeModules, Platform } from 'react-native';

/** A portable collection of non-destructive edits. Analysis and pixels always belong to the photo, never a preset. */
export interface EditPreset extends ProjectRecipe {
  version: 1;
  id: string;
  name: string;
  /** Optional note that helps identify a look in the preset library. */
  description?: string;
  createdAt: number;
}

type PresetStoreModule = {
  listPresets(): Promise<string>;
  writePresets(json: string): Promise<void>;
};

const store = NativeModules.ProjectStore as (PresetStoreModule | undefined);
const PRESET_SECTIONS = new Set(['section-mood', 'section-adjustments', 'section-beauty']);

function native(): PresetStoreModule {
  if (Platform.OS !== 'android' || !store?.listPresets || !store.writePresets) {
    throw new Error('The installed app is older than this code. Rebuild it with npm run dev:android.');
  }
  return store;
}

/** Controls that are meaningful on another photo. Tools and transforms are deliberately left out. */
const reusableControls = (): Exclude<EditControl, { kind: 'group' | 'mixed' | 'tool' | 'picker' }>[] =>
  EDIT_SECTIONS.filter(section => PRESET_SECTIONS.has(section.key)).flatMap(section =>
    leafControls(section.controls).filter(
      (control): control is Exclude<EditControl, { kind: 'group' | 'mixed' | 'tool' | 'picker' }> =>
        control.kind !== 'tool' && control.kind !== 'picker',
    ),
  );

/** The controls a preset owns when it is applied. Missing values turn those controls off. */
export const presetControlKeys = () => new Set(reusableControls().map(control => control.key));

/** Removes values aimed at a particular person: those ids only make sense on the photo where they were made. */
export function portableRecipe(recipe: ProjectRecipe): ProjectRecipe {
  const keys = presetControlKeys();
  const adjustments: Record<string, AdjustmentValue> = {};
  for (const [key, value] of Object.entries(recipe.adjustments)) {
    if (keys.has(key) && !key.includes(':person:')) adjustments[key] = Array.isArray(value) ? [...value] : value;
  }
  return { adjustments, appliedActions: recipe.appliedActions.filter(key => keys.has(key)) };
}

export const listPresets = async (): Promise<EditPreset[]> => {
  const parsed: unknown = JSON.parse(await native().listPresets());
  if (!Array.isArray(parsed)) return [];
  return parsed.filter(isPreset).map(preset => ({ ...preset, ...portableRecipe(preset) }));
};

export const writePresets = (presets: readonly EditPreset[]) => native().writePresets(JSON.stringify(presets));

export function makePreset(name: string, recipe: ProjectRecipe, description?: string): EditPreset {
  return {
    version: 1,
    id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    name: name.trim(),
    ...(description?.trim() ? { description: description.trim() } : {}),
    createdAt: Date.now(),
    ...portableRecipe(recipe),
  };
}

const isPreset = (value: any): value is EditPreset =>
  value &&
  value.version === 1 &&
  typeof value.id === 'string' &&
  typeof value.name === 'string' &&
  value.adjustments &&
  Array.isArray(value.appliedActions);
