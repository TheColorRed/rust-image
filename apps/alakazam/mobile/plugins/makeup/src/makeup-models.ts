import { faceMasks } from '@alakazam/plugin-toolkit';

/** Face-analysis model roles required by this plugin's face effects. */
export const MAKEUP_MODELS = ['person-detection', 'face-parsing', 'face-landmarks'] as const;

/** Build the face masks used by the makeup controls from the app's shared face analysis. */
export const makeupMasks = faceMasks;
