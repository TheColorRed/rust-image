/** The keys of the controls that the editor treats specially. They are here, and not with the controls, so the state that
 * reads them does not depend on the controls and is not re-run when one is edited. */
export const BLEMISH_TOOL_KEY = 'action-blemish';
export const RED_EYE_TOOL_KEY = 'action-red-eye';
export const CROP_TOOL_KEY = 'action-crop';
export const GRADIENT_KEY = 'overlay-gradient';
/** The slider that holds the gradient's settings. It is what is open (focused) while the gradient is, and the handles on the photo show for it. */
export const GRADIENT_SETTINGS_KEY = `${GRADIENT_KEY}-settings`;
export const STRIPE_KEY = 'overlay-stripes';
/** The slider that holds the stripes' settings: what is open (focused) while the stripes are, and what the words dialog saves into. */
export const STRIPE_SETTINGS_KEY = `${STRIPE_KEY}-settings`;
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
