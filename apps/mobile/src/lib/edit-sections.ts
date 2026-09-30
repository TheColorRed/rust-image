import { EffectSpec, PhotoFilterPreset, type AbraImageLike } from '@alakazam/mobile';

/** A control whose value is applied once the user releases the slider. */
export type SliderControl = {
  kind: 'slider';
  key: string;
  label: string;
  min: number;
  max: number;
  step: number;
  /** The value the slider snaps back to after each release; each drag applies a fresh delta. */
  defaultValue: number;
  apply: (image: AbraImageLike, value: number) => void;
  /** When set, dragging previews this effect live (GPU where available) before the release commits it via `apply`. */
  live?: (value: number) => EffectSpec;
};

/** A control that applies immediately on tap, with no parameters. */
export type ActionControl = {
  kind: 'action';
  key: string;
  label: string;
  apply: (image: AbraImageLike) => void;
  /**
   * Controls sharing a group are mutually exclusive: applying one clears any other applied
   * control in the same group (across all sections). Ungrouped controls toggle independently.
   */
  group?: string;
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

export type EditControl = SliderControl | ActionControl | ToolControl;

export type EditSection = {
  key: string;
  label: string;
  controls: EditControl[];
  /** Show each control as a thumbnail of the effect applied, instead of a plain text chip. */
  previewThumbnails?: boolean;
};

/** Key of the blemish tool control; the editor dispatches on it to render the reticle and controls. */
export const BLEMISH_TOOL_KEY = 'action-blemish';

// Ordered by what a casual, non-designer social-media poster reaches for first: filters, then
// basic brightness/color tweaks, then face retouching — with the more technical, restoration-style
// tools (Detail, Cleanup) pushed to the back since that audience rarely needs them.
export const EDIT_SECTIONS: EditSection[] = [
  {
    key: 'section-colorization',
    label: 'Colorization',
    previewThumbnails: true,
    controls: [
      {
        kind: 'action',
        key: 'action-colorize-warm',
        label: 'Warm',
        group: 'colorize',
        apply: image => image.photoFilter(PhotoFilterPreset.Warming, 0.35),
      },
      {
        kind: 'action',
        key: 'action-colorize-cold',
        label: 'Cold',
        group: 'colorize',
        apply: image => image.photoFilter(PhotoFilterPreset.Cooling, 0.35),
      },
      {
        kind: 'action',
        key: 'action-colorize-sunny',
        label: 'Sunny',
        group: 'colorize',
        apply: image => {
          image.photoFilter(PhotoFilterPreset.Warming, 0.2);
          image.vibrance(20);
          image.applyEffect(EffectSpec.Brightness.new({ amount: 8 }));
        },
      },
      {
        kind: 'action',
        key: 'action-colorize-cloudy',
        label: 'Cloudy',
        group: 'colorize',
        apply: image => {
          image.photoFilter(PhotoFilterPreset.Cooling, 0.2);
          image.applyEffect(EffectSpec.Saturation.new({ amount: -20 }));
          image.applyEffect(EffectSpec.Brightness.new({ amount: -6 }));
        },
      },
      {
        kind: 'action',
        key: 'action-colorize-sepia',
        label: 'Sepia',
        group: 'colorize',
        apply: image => image.photoFilter(PhotoFilterPreset.Sepia, 0.35),
      },
      {
        kind: 'action',
        key: 'action-colorize-vibrant',
        label: 'Vibrant',
        group: 'colorize',
        apply: image => {
          image.applyEffect(EffectSpec.Saturation.new({ amount: 30 }));
          image.vibrance(30);
        },
      },
      {
        kind: 'action',
        key: 'action-colorize-fade',
        label: 'Fade',
        group: 'colorize',
        apply: image => {
          image.applyEffect(EffectSpec.Contrast.new({ amount: -20 }));
          image.applyEffect(EffectSpec.Saturation.new({ amount: -15 }));
        },
      },
      {
        kind: 'action',
        key: 'action-colorize-moody',
        label: 'Moody',
        group: 'colorize',
        apply: image => {
          image.photoFilter(PhotoFilterPreset.Cooling, 0.25);
          image.applyEffect(EffectSpec.Contrast.new({ amount: 15 }));
          image.applyEffect(EffectSpec.Brightness.new({ amount: -10 }));
        },
      },
      {
        kind: 'action',
        key: 'action-colorize-golden-hour',
        label: 'Golden Hour',
        group: 'colorize',
        apply: image => {
          image.photoFilter(PhotoFilterPreset.Warming, 0.45);
          image.applyEffect(EffectSpec.Brightness.new({ amount: 12 }));
        },
      },
      {
        kind: 'action',
        key: 'action-colorize-dreamy',
        label: 'Dreamy',
        group: 'colorize',
        apply: image => {
          image.photoFilter(PhotoFilterPreset.Warming, 0.15);
          image.applyEffect(EffectSpec.Contrast.new({ amount: -15 }));
          image.applyEffect(EffectSpec.Saturation.new({ amount: -10 }));
        },
      },
      {
        kind: 'action',
        key: 'action-colorize-noir',
        label: 'Noir',
        group: 'colorize',
        apply: image => {
          image.grayscale();
          image.applyEffect(EffectSpec.Contrast.new({ amount: 20 }));
        },
      },
      {
        kind: 'action',
        key: 'action-colorize-vintage',
        label: 'Vintage',
        group: 'colorize',
        apply: image => {
          image.photoFilter(PhotoFilterPreset.Sepia, 0.2);
          image.applyEffect(EffectSpec.Contrast.new({ amount: -10 }));
          image.applyEffect(EffectSpec.Saturation.new({ amount: -20 }));
        },
      },
      {
        kind: 'action',
        key: 'action-colorize-ocean',
        label: 'Ocean',
        group: 'colorize',
        apply: image => image.photoFilter(PhotoFilterPreset.Underwater, 0.35),
      },
      {
        kind: 'action',
        key: 'action-colorize-forest',
        label: 'Forest',
        group: 'colorize',
        apply: image => image.photoFilter(PhotoFilterPreset.DeepEmerald, 0.3),
      },
      {
        kind: 'action',
        key: 'action-colorize-frost',
        label: 'Frost',
        group: 'colorize',
        apply: image => image.photoFilter(PhotoFilterPreset.CoolingDark, 0.3),
      },
      {
        kind: 'action',
        key: 'action-colorize-neon',
        label: 'Neon',
        group: 'colorize',
        apply: image => image.photoFilter(PhotoFilterPreset.Magenta, 0.25),
      },
    ],
  },
  {
    key: 'section-adjustments',
    label: 'Adjustments',
    controls: [
      {
        kind: 'slider',
        key: 'action-brightness',
        label: 'Brightness',
        min: -100,
        max: 100,
        step: 1,
        defaultValue: 0,
        apply: (image, value) => image.applyEffect(EffectSpec.Brightness.new({ amount: Math.round(value) })),
        live: value => EffectSpec.Brightness.new({ amount: Math.round(value) }),
      },
      {
        kind: 'slider',
        key: 'action-contrast',
        label: 'Contrast',
        min: -100,
        max: 100,
        step: 1,
        defaultValue: 0,
        apply: (image, value) => image.applyEffect(EffectSpec.Contrast.new({ amount: Math.round(value) })),
        live: value => EffectSpec.Contrast.new({ amount: Math.round(value) }),
      },
      {
        kind: 'slider',
        key: 'action-saturation',
        label: 'Saturation',
        min: -100,
        max: 100,
        step: 1,
        defaultValue: 0,
        apply: (image, value) => image.applyEffect(EffectSpec.Saturation.new({ amount: Math.round(value) })),
        live: value => EffectSpec.Saturation.new({ amount: Math.round(value) }),
      },
      {
        kind: 'slider',
        key: 'action-exposure',
        label: 'Exposure',
        min: -5,
        max: 5,
        step: 0.1,
        defaultValue: 0,
        apply: (image, value) => image.applyEffect(EffectSpec.Exposure.new({ exposure: value, offset: 0.0, gammaCorrection: 1.0 })),
        live: value => EffectSpec.Exposure.new({ exposure: value, offset: 0.0, gammaCorrection: 1.0 }),
      },
      {
        kind: 'slider',
        key: 'action-vibrance',
        label: 'Vibrance',
        min: -100,
        max: 100,
        step: 1,
        defaultValue: 0,
        apply: (image, value) => image.vibrance(value),
      },
    ],
  },
  {
    key: 'section-beauty',
    label: 'Beauty',
    controls: [
      {
        kind: 'slider',
        key: 'action-skin-smooth',
        label: 'Skin Smooth',
        min: 0,
        max: 1,
        step: 0.1,
        defaultValue: 0,
        apply: (image, value) => image.smoothSkin(value),
      },
      { kind: 'tool', key: BLEMISH_TOOL_KEY, label: 'Blemish' },
    ],
  },
  {
    key: 'section-transform',
    label: 'Transform',
    previewThumbnails: true,
    controls: [
      { kind: 'action', key: 'action-rotate-left', label: 'Rotate Left', apply: image => image.rotate(-90) },
      { kind: 'action', key: 'action-rotate-right', label: 'Rotate Right', apply: image => image.rotate(90) },
      {
        kind: 'action',
        key: 'action-flip-horizontal',
        label: 'Flip Horizontal',
        apply: image => image.flipHorizontal(),
      },
      { kind: 'action', key: 'action-flip-vertical', label: 'Flip Vertical', apply: image => image.flipVertical() },
    ],
  },
  {
    key: 'section-sharpen',
    label: 'Sharpen',
    previewThumbnails: true,
    controls: [{ kind: 'action', key: 'action-sharpen', label: 'Sharpen', apply: image => image.sharpen() }],
  },
  {
    key: 'section-effects',
    label: 'Effects',
    previewThumbnails: true,
    controls: [
      { kind: 'action', key: 'action-grayscale', label: 'Grayscale', apply: image => image.grayscale() },
      { kind: 'action', key: 'action-invert', label: 'Invert', apply: image => image.invert() },
      { kind: 'action', key: 'action-auto-tone', label: 'Auto Tone', apply: image => image.autoTone() },
      { kind: 'action', key: 'action-auto-color', label: 'Auto Color', apply: image => image.autoColor() },
      { kind: 'action', key: 'action-threshold', label: 'Threshold', apply: image => image.threshold(128) },
      { kind: 'action', key: 'action-posterize', label: 'Posterize', apply: image => image.posterize(6) },
    ],
  },
  {
    key: 'section-blur',
    label: 'Blur',
    controls: [
      {
        kind: 'slider',
        key: 'action-gaussian-blur',
        label: 'Gaussian',
        min: 0,
        max: 40,
        step: 1,
        defaultValue: 0,
        apply: (image, value) => image.applyEffect(EffectSpec.GaussianBlur.new({ radius: value })),
      },
      {
        kind: 'slider',
        key: 'action-box-blur',
        label: 'Box',
        min: 0,
        max: 40,
        step: 1,
        defaultValue: 0,
        apply: (image, value) => image.boxBlur(value),
      },
    ],
  },
  {
    key: 'section-detail',
    label: 'Detail',
    previewThumbnails: true,
    controls: [{ kind: 'action', key: 'action-smooth', label: 'Smooth', apply: image => image.smooth() }],
  },
  {
    key: 'section-cleanup',
    label: 'Cleanup',
    controls: [
      {
        kind: 'slider',
        key: 'action-median',
        label: 'Median',
        min: 0,
        max: 8,
        step: 1,
        defaultValue: 0,
        apply: (image, value) => image.median(value),
      },
      { kind: 'action', key: 'action-despeckle', label: 'Despeckle', apply: image => image.despeckle() },
    ],
  },
];

export const findSection = (key: string): EditSection => EDIT_SECTIONS.find(section => section.key === key) ?? EDIT_SECTIONS[0];
