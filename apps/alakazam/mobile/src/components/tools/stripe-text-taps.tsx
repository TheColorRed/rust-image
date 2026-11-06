import { useMemo } from 'react';
import { Pressable, StyleSheet, View } from 'react-native';
import { useObservable } from 'react-rx';
import { STRIPE_PLACE_DEFAULTS, stripeTextBoxes, stripeValues, type StripePlace, type StripeTextBox } from '@/src/effects/stripe-overlay';
import { computePreviewScreenPoint } from '@/src/lib/editor/preview-coordinates';
import { leafControls, sliderValues, type SliderControl } from '@/src/lib/project/edit-sections';
import { STRIPE_KEY, STRIPE_SETTINGS_KEY } from '@/src/lib/project/edit-sections/keys';
import { stripeControl } from '@/src/lib/project/edit-sections/overlays';
import { stripeTextRequested } from '@/src/state/commands';
import { adjustments, adjustments$ } from '@/src/state/edits';
import {
  previewBox,
  previewBox$,
  previewRotationValue,
  previewRotationValue$,
  previewScaleValue,
  previewScaleValue$,
  previewTranslateValue,
  previewTranslateValue$,
} from '@/src/state/gestures';
import { currentImage, currentImage$ } from '@/src/state/session';

/** The least a place to tap may be across, in dp, so a short word is still easy to hit. */
const MIN_TAP = 44;

/** The sliders of the title bars by key, so the saved value of each can be read from the edits. */
const sliders = leafControls([stripeControl]).filter((control): control is SliderControl => control.kind === 'slider');
const sliderOf = (key: string) => sliders.find(control => control.key === key)!;

/**
 * Places over the main text and the sub text of the open title bars that open that text's dialog when tapped, drawn over the photo where the
 * text is (turned as the text is, and following the zoom and pan). Where the text is comes from the same layout that draws it
 * (`stripeTextBoxes`), so the two always agree. A touch anywhere else goes to the preview, so the photo can still be pinched and panned.
 */
export function StripeTextTaps() {
  const edits = useObservable(adjustments$, adjustments.value);
  const box = useObservable(previewBox$, previewBox.value);
  const scale = useObservable(previewScaleValue$, previewScaleValue.value);
  const translate = useObservable(previewTranslateValue$, previewTranslateValue.value);
  const turn = useObservable(previewRotationValue$, previewRotationValue.value);
  const image = useObservable(currentImage$, currentImage.value);

  // Where the text is on the photo, worked out again only when an edit or the photo changes (it makes the text pictures, which a drag of the
  // preview must not do for every move).
  const boxes = useMemo(() => {
    if (!image || image.width() === 0 || image.height() === 0) return null;
    const value = (key: string) => sliderValues(sliderOf(key), edits[key])[0];
    const [gap, shift] = sliderValues(sliderOf(`${STRIPE_KEY}-align-settings`), edits[`${STRIPE_KEY}-align-settings`]);
    const placed: StripePlace = {
      ...STRIPE_PLACE_DEFAULTS,
      position: value(`${STRIPE_KEY}-position`),
      angle: value(`${STRIPE_KEY}-angle`),
      size: value(`${STRIPE_KEY}-size`),
      padding: value(`${STRIPE_KEY}-padding`),
      gap,
      shift,
    };
    const values = stripeValues(sliderValues(sliderOf(STRIPE_SETTINGS_KEY), edits[STRIPE_SETTINGS_KEY]));
    return stripeTextBoxes(image.width(), image.height(), values, placed);
  }, [edits, image]);

  if (!box || !image || !boxes) return null;

  // A text's rectangle on the screen: its middle, its size and how much it is turned, from where three of its corners land.
  const onScreen = (text: StripeTextBox) => {
    const radians = (text.angle * Math.PI) / 180;
    const along = [Math.cos(radians), Math.sin(radians)] as const;
    const down = [-Math.sin(radians), Math.cos(radians)] as const;
    const corner = (sideways: number, downwards: number) =>
      computePreviewScreenPoint(
        image.width(),
        image.height(),
        box,
        scale,
        translate,
        {
          x: (text.x + along[0] * sideways * (text.width / 2) + down[0] * downwards * (text.height / 2)) / image.width(),
          y: (text.y + along[1] * sideways * (text.width / 2) + down[1] * downwards * (text.height / 2)) / image.height(),
        },
        turn,
      );
    const [topLeft, topRight, bottomLeft] = [corner(-1, -1), corner(1, -1), corner(-1, 1)];
    return {
      x: (topRight.x + bottomLeft.x) / 2,
      y: (topRight.y + bottomLeft.y) / 2,
      width: Math.hypot(topRight.x - topLeft.x, topRight.y - topLeft.y),
      height: Math.hypot(bottomLeft.x - topLeft.x, bottomLeft.y - topLeft.y),
      degrees: (Math.atan2(topRight.y - topLeft.y, topRight.x - topLeft.x) * 180) / Math.PI,
    };
  };

  const places = (
    [
      ['main', 'Main text', boxes.main],
      ['sub', 'Sub text', boxes.sub],
    ] as const
  ).flatMap(([which, label, text]) => (text ? [{ which, label, ...onScreen(text) }] : []));

  // The layer is clipped to the preview, as the gradient's is, so a place cannot be under the top bar.
  return (
    <View pointerEvents="box-none" style={[StyleSheet.absoluteFill, styles.layer]}>
      {places.map(place => {
        const width = Math.max(place.width, MIN_TAP);
        const height = Math.max(place.height, MIN_TAP);
        return (
          <Pressable
            key={place.which}
            accessibilityRole="button"
            accessibilityLabel={`Edit ${place.label.toLowerCase()}`}
            onPress={() => stripeTextRequested.next(place.which)}
            style={{ position: 'absolute', left: place.x - width / 2, top: place.y - height / 2, width, height, transform: [{ rotate: `${place.degrees}deg` }] }}
          />
        );
      })}
    </View>
  );
}

const styles = StyleSheet.create({
  layer: { overflow: 'hidden' },
});
