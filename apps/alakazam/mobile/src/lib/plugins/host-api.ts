/**
 * What the app offers a plugin script, as the module `@alakazam/host`. A script cannot import the app's own code, so this is the whole
 * of what it can use of the app, besides the Abra bindings (`@abra/core/...`). It is the single list: the loader hands scripts exactly
 * this module, and a plugin's TypeScript is checked against it.
 *
 * Keep it small and stable. Anything added here is something every plugin can depend on.
 */
export { derivedFor } from '@/src/lib/editor/editor-preloads';
export { HAIR_MODELS, MAKEUP_MODELS, belongsTo } from '@/src/lib/ai/faces';
export { owning, release } from '@/src/lib/native/native-handles';
export { paintSheet, type Painting } from '@/src/lib/beauty/layers';
export { makeupMasks } from '@/src/lib/beauty/makeup-masks';
export { swatchesOf } from '@/src/lib/project/edit-sections/swatches';
export type { EditControl, SliderControl, ToolContext } from '@/src/lib/project/edit-sections';

/**
 * What a control script of a plugin exports as its default: a function that returns the control. The plugin's `plugin.yml` says where
 * the control goes and what it is called, so the control does not.
 */
export type ControlScript = () => import('@/src/lib/project/edit-sections').EditControl;
