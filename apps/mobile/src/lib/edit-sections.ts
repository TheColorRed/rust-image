import { type AbraImageLike, type EffectSpec } from '@alakazam/mobile';
import { adjustments } from './edit-sections/adjustments';
import { BLEMISH_TOOL_KEY, beauty } from './edit-sections/beauty';
import { blur } from './edit-sections/blur';
import { cleanup } from './edit-sections/cleanup';
import { colorization } from './edit-sections/colorization';
import { detail } from './edit-sections/detail';
import { effects } from './edit-sections/effects';
import { sharpen } from './edit-sections/sharpen';
import { transform } from './edit-sections/transform';

/** A control whose value is applied once the user releases the slider. */
export type SliderControl = {
  kind: 'slider';
  key: string;
  label: string;
  min: number;
  max: number;
  step?: number;
  /** The value the slider snaps back to after each release; each drag applies a fresh delta. */
  defaultValue: number;
  /**
   * Whether selecting the slider applies `defaultValue` at once, and the control counts as applied whenever it has a value.
   * On by default, for effects with no neutral value (Threshold has no "off" setting). Set it to `false` when `defaultValue`
   * means "no change" (Brightness at 0): the control then counts as applied only once it differs from `defaultValue`.
   */
  applyOnSelect?: boolean;
  apply: (image: AbraImageLike, value: number) => void;
  /** When set, dragging previews this effect live (GPU where available) before the release commits it via `apply`. */
  live?: (value: number) => EffectSpec;
};

/** A control that applies immediately on tap, with no parameters. */
export type ActionControl = {
  kind: 'action';
  key: string;
  label: string;
  apply?: (image: AbraImageLike) => void;
  /**
   * Controls sharing a group are mutually exclusive: applying one clears any other applied
   * control in the same group (across all sections). Ungrouped controls toggle independently.
   */
  group?: string;
  /** When set, tapping previews this effect instantly (GPU where available) while the full-resolution edit replays behind it. */
  live?: () => EffectSpec;
};

/**
 * A control with no value of its own to configure — tapping it hands off to a custom, on-image
 * interaction of the edit screen's own choosing (e.g. the blemish tool's reticle + button, or a
 * future gradient tool's tap-two-points-then-apply), rather than a slider or an instant whole-
 * image effect. Unlike `ActionControl` it has no `apply`, since what it does isn't reusable across
 * controls — the screen dispatches on `key` to render and run the right one.
 */
export type ToolControl = {
  kind: 'tool';
  key: string;
  label: string;
};

/** Applies an action to `image`: its own `apply`, or else its live effect, which is the same definition. */
export const applyAction = (control: ActionControl, image: AbraImageLike) => {
  if (control.apply) control.apply(image);
  else if (control.live) image.applyEffect(control.live());
};

/** Whether a slider's effect is on, given its committed value (undefined when it has none). */
export const appliesOnSelect = (control: SliderControl) => control.applyOnSelect !== false;

export const isSliderApplied = (control: SliderControl, value: number | undefined) =>
  appliesOnSelect(control) ? value !== undefined : (value ?? control.defaultValue) !== control.defaultValue;

export type EditControl = SliderControl | ActionControl | ToolControl;

export type EditSection = {
  key: string;
  label: string;
  controls: EditControl[];
  /** Show each control as a thumbnail of the effect applied, instead of a plain text chip. */
  previewThumbnails?: boolean;
};

/** Key of the blemish tool control; the editor dispatches on it to render the reticle and controls. */
export { BLEMISH_TOOL_KEY };

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

export const findSection = (key: string): EditSection => EDIT_SECTIONS.find(section => section.key === key) ?? EDIT_SECTIONS[0];
