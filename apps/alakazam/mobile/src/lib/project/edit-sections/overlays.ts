import {
  GRADIENT_AREA,
  GRADIENT_AT,
  GRADIENT_BLENDS,
  GRADIENT_CUSTOM,
  GRADIENT_LOOKS,
  GRADIENT_STRENGTH,
  PHOTO_HARMONIES,
  gradientFromPhoto,
  gradientOverlay,
  withCustomGradient,
} from '@/src/effects/gradient-overlay';
import {
  STRIPE_PLACE_DEFAULTS,
  STRIPE_AT,
  STRIPE_DEFAULTS,
  STRIPE_LOOKS,
  stripeOverlay,
} from '@/src/effects/stripe-overlay';
import { type ImageLike } from '@abra/core/image';
import Type from 'lucide-react-native/icons/type';
import Image from 'lucide-react-native/icons/image';
import Mountain from 'lucide-react-native/icons/mountain';
import Palette from 'lucide-react-native/icons/palette';
import Pipette from 'lucide-react-native/icons/pipette';
import User from 'lucide-react-native/icons/user';
import { subjectMask } from '@/src/lib/ai/subject';
import { gradientChoiceRequested, stripeTextRequested } from '@/src/state/commands';
import { EditSection, MixedControl, SliderControl, type ToolContext } from '@/src/lib/project/edit-sections';
import {
  GRADIENT_KEY,
  GRADIENT_SETTINGS_KEY as SETTINGS_KEY,
  STRIPE_KEY,
  STRIPE_SETTINGS_KEY,
} from '@/src/lib/project/edit-sections/keys';

/** Things people lay over their photo. The gradient is the first. */
export const gradientControl: MixedControl = {
  kind: 'mixed',
  key: GRADIENT_KEY,
  label: 'Gradient',
  thumb: photo => gradientOverlay(photo, []),
  tool: (photo, [values], { preloaded }) => gradientOverlay(photo, values, preloaded),
  controls: [
    {
      kind: 'slider',
      key: SETTINGS_KEY,
      label: 'Settings',
      // An array of one, so the control keeps every value it was given (the choices and the handles), not only the slider's.
      sliderLabel: ['Strength'],
      display: 'tabulated',
      pillFormat: ['percent'],
      min: [0],
      max: [1],
      step: [0.01],
      defaultValue: [GRADIENT_STRENGTH],
      applyOnSelect: true,
      // The photo remembers the effect and the GPU draws it, so it is made on the full-size photo and stays sharp when zoomed.
      deferred: true,
      // The cutout of the people, for a gradient that goes only behind them or only on them. It is found once for the photo, kept with the
      // project, and in most cases is already made when the photo opens (see `buildMasks`), so a gradient on the whole photo never waits for it.
      preload: subjectMask,
    } satisfies SliderControl<number>,
    {
      kind: 'picker',
      key: `${GRADIENT_KEY}-look`,
      label: 'Colors',
      selection: 'one',
      commitTo: { slider: SETTINGS_KEY, index: GRADIENT_AT.look },
      after: -1,
      items: [
        ...GRADIENT_LOOKS.map((look, value) => ({
          key: `${GRADIENT_KEY}-look-${value}`,
          label: look.label,
          picture: { kind: 'rendered' as const },
          value,
          // Only the colors change: the strength, the blend and the handles are as the other tabs have them now.
          thumb: (photo: ImageLike, { values = [] }: ToolContext) =>
            gradientOverlay(photo, forThumb(withValue(values, GRADIENT_AT.look, value))),
        })),
        {
          // A gradient from the photo's most colorful color to the color opposite it. Tapping it works them out and saves them as the person's
          // own gradient (which Custom then opens to change), so the colors are kept with the project and never worked out again.
          key: `${GRADIENT_KEY}-look-photo`,
          label: 'From photo',
          picture: { kind: 'icon' as const, icon: Pipette },
          // Opens a row of the color schemes to build the gradient with, each shown on the photo.
          items: PHOTO_HARMONIES.map(({ key, label }) => ({
            key: `${GRADIENT_KEY}-look-photo-${key}`,
            label,
            picture: { kind: 'rendered' as const },
            pick: () => gradientChoiceRequested.next({ kind: 'photo', scheme: key }),
            // The scheme as it would come out on this photo, with the strength, the blend and the handles as they are now.
            thumb: (photo: ImageLike, { values = [] }: ToolContext) => {
              const made = gradientFromPhoto(photo, key);
              gradientOverlay(photo, forThumb(made ? withCustomGradient(values, made) : values));
            },
          })),
        },
        {
          // The person's own gradient. Tapping it opens the gradient editor, which saves the choice when they press OK; the open gradient
          // answers the request (see `GradientHandles`).
          key: `${GRADIENT_KEY}-look-custom`,
          label: 'Customize',
          picture: { kind: 'icon' as const, icon: Palette },
          value: GRADIENT_CUSTOM,
          pick: () => gradientChoiceRequested.next({ kind: 'custom' }),
        },
      ],
    },
    {
      kind: 'picker',
      key: `${GRADIENT_KEY}-blend`,
      label: 'Blend',
      selection: 'one',
      commitTo: { slider: SETTINGS_KEY, index: GRADIENT_AT.blend },
      after: 0,
      items: GRADIENT_BLENDS.map((blend, value) => ({
        key: `${GRADIENT_KEY}-blend-${value}`,
        label: blend.label,
        picture: { kind: 'rendered' as const },
        value,
        thumb: (photo, { values = [] }) =>
          gradientOverlay(photo, forThumb(withValue(values, GRADIENT_AT.blend, value))),
      })),
    },
    {
      kind: 'picker',
      key: `${GRADIENT_KEY}-area`,
      label: 'Area',
      selection: 'one',
      commitTo: { slider: SETTINGS_KEY, index: GRADIENT_AT.area },
      after: 0,
      // Each choice is made by the open gradient, which first asks for the models that find the people when they are not on the device.
      items: (
        [
          ['Whole photo', GRADIENT_AREA.whole, Image],
          ['Background', GRADIENT_AREA.background, Mountain],
          ['People', GRADIENT_AREA.people, User],
        ] as const
      ).map(([label, value, icon]) => ({
        key: `${GRADIENT_KEY}-area-${value}`,
        label,
        picture: { kind: 'icon' as const, icon },
        value,
        pick: () => gradientChoiceRequested.next({ kind: 'area', area: value }),
      })),
    },
  ],
};

/** The values for a thumbnail, which always shows the gradient on the whole photo: it has no cutout of the people to limit it to. */
const forThumb = (values: number[]): number[] => withValue(values, GRADIENT_AT.area, GRADIENT_AREA.whole);

/** `values` with the one at `index` set. A place left empty reads as missing, so `gradientOverlay` uses its default there. */
const withValue = (values: number[], index: number, value: number): number[] => {
  const next = [...values];
  next[index] = value;
  return next;
};

/** A slider of the stripes that holds one share or angle. They are one control each, so the root and the groups under it show them as tabs. */
const stripeSlider = (key: string, label: string, min: number, max: number, step: number, defaultValue: number, pill: 'percent' | 'plain') =>
  ({ kind: 'slider', key: `${STRIPE_KEY}-${key}`, label, min, max, step, defaultValue, pillFormat: pill, applyOnSelect: true, deferred: true }) satisfies SliderControl<number>;

/**
 * Tilted stripes with text on them, like the title cards of a TV show. The text is set in dialogs (see `StripeWordsDialog`). The row shows the
 * text, the colors, Move & Turn (with its Position, Rotation and Sub Align), then the Strength, Size and Padding. The Strength slider holds what
 * the pickers save (the colors, the switches and the text), so a change of text or colors is one step to undo like any other.
 */
export const stripeControl: MixedControl = {
  kind: 'mixed',
  key: STRIPE_KEY,
  label: 'Title Bars',
  thumb: photo => stripeOverlay(photo, []),
  // The values of each slider, in the order they are found: the position, the angle, the sub stripe's gap and shift, the strength (with what the
  // pickers saved after it), the size and the padding.
  tool: (photo, [[position], [angle], [gap, shift], values, [size], [padding]]) => stripeOverlay(photo, values, { position, angle, gap, shift, size, padding }),
  controls: [
    {
      kind: 'picker',
      key: `${STRIPE_KEY}-text`,
      label: 'Text',
      selection: 'one',
      // Each opens its own dialog, which the open stripes answer; the text is saved when OK is pressed.
      items: [
        { key: `${STRIPE_KEY}-text-main`, label: 'Main Text', picture: { kind: 'icon' as const, icon: Type }, pick: () => stripeTextRequested.next('main') },
        { key: `${STRIPE_KEY}-text-sub`, label: 'Sub Text', picture: { kind: 'icon' as const, icon: Type }, pick: () => stripeTextRequested.next('sub') },
      ],
    },
    {
      kind: 'picker',
      key: `${STRIPE_KEY}-look`,
      label: 'Color',
      selection: 'one',
      commitTo: { slider: STRIPE_SETTINGS_KEY, index: STRIPE_AT.look },
      items: STRIPE_LOOKS.map((look, value) => ({
        key: `${STRIPE_KEY}-look-${value}`,
        label: look.label,
        picture: { kind: 'rendered' as const },
        value,
        // Only the colors change: the other settings are as they are now.
        thumb: (photo: ImageLike, { values = [] }: ToolContext) => stripeOverlay(photo, withValue(values, STRIPE_AT.look, value)),
      })),
    },
    {
      kind: 'mixed',
      key: `${STRIPE_KEY}-transform`,
      label: 'Move & Turn',
      controls: [
        stripeSlider('position', 'Position', 0, 1, 0.01, STRIPE_PLACE_DEFAULTS.position, 'percent'),
        stripeSlider('angle', 'Rotation', -45, 45, 1, STRIPE_PLACE_DEFAULTS.angle, 'plain'),
        {
          // Where the sub stripe goes, a tab of Move & Turn. Both are shares of the room it has on the photo, from 0 to 1.
          kind: 'mixed',
          key: `${STRIPE_KEY}-align`,
          label: 'Sub Align',
          controls: [
            {
              kind: 'slider',
              key: `${STRIPE_KEY}-align-settings`,
              label: 'Settings',
              sliderLabel: ['Up/down', 'Left/right'],
              display: 'tabulated',
              pillFormat: ['percent', 'percent'],
              min: [-1, 0],
              max: [1, 1],
              step: [0.01, 0.01],
              defaultValue: [STRIPE_PLACE_DEFAULTS.gap, STRIPE_PLACE_DEFAULTS.shift],
              applyOnSelect: true,
              deferred: true,
            } satisfies SliderControl<number>,
          ],
        },
      ],
    },
    // The photo remembers the effect and the GPU draws it, so it is made on the full-size photo and stays sharp when zoomed.
    { ...stripeSlider('strength', 'Strength', 0, 1, 0.01, STRIPE_DEFAULTS.strength, 'percent'), key: STRIPE_SETTINGS_KEY },
    stripeSlider('size', 'Size', 0, 1, 0.01, STRIPE_PLACE_DEFAULTS.size, 'percent'),
    stripeSlider('padding', 'Padding', 0, 1, 0.01, STRIPE_PLACE_DEFAULTS.padding, 'percent'),
  ],
};

export const overlays: EditSection = {
  key: 'section-overlays',
  label: 'Overlays',
  previewThumbnails: true,
  controls: [gradientControl, stripeControl],
};
