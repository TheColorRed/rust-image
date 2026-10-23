/** The keys of the controls that the editor treats specially. They are here, and not with the controls, so the state that
 * reads them does not depend on the controls and is not re-run when one is edited. */
export const BLEMISH_TOOL_KEY = 'action-blemish';
export const RED_EYE_TOOL_KEY = 'action-red-eye';
export const CROP_TOOL_KEY = 'action-crop';
export const OBJECT_REMOVAL_TOOL_KEY = 'action-object-removal';
/** The adjustment key that holds the area drawn for a Color Splash color, `index` from 0. */
export const colorAreaKey = (index: number) => `area-color-splash-${index + 1}`;
export const SKIN_SMOOTH_KEY = 'action-skin-smooth';
export const SKIN_EVEN_OUT_KEY = 'action-skin-even-out';
export const SKIN_TAN_KEY = 'action-skin-tan';
export const SKIN_TONE_KEY = 'action-skin-tone';
export const SKIN_GROUP_KEY = 'group-skin';
export const HAIR_GROUP_KEY = 'group-hair';
export const HAIR_COLOR_KEY = 'slider-hair-color';
export const TEETH_WHITENING_KEY = 'slider-teeth-whitening';
