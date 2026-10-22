import type { AbraImageLike } from '@alakazam/mobile';
import { useEffect, useMemo } from 'react';
import { useObservable } from 'react-rx';
import { StyleSheet } from 'react-native';
import Svg, { Path } from 'react-native-svg';
import { useAreaTarget } from '@/src/components/tools/draw-area';
import { combineArea } from '@/src/lib/native/area-mask';
import { areaPoints, encodeAreaRings, findSection, leafControls, sliderSpecs, sliderValues } from '@/src/lib/project/edit-sections';
import { commitColorArea } from '@/src/lib/editor/editor-history';
import {
  activeSectionKey,
  activeSectionKey$,
  adjustments,
  adjustments$,
  areaOutlinesHidden,
  areaOutlinesHidden$,
  focusedControlKey,
  focusedControlKey$,
  openSliderState,
  openSliderState$,
} from '@/src/state/edits';
import { areaRings, areas, areas$, areaTarget, areaTarget$, setArea, type DrawMode, type DrawnPoint } from '@/src/state/draw-area';
import { currentImage } from '@/src/state/session';
import { previewScaleSettled$, previewScaleValue, type PreviewBox } from '@/src/state/gestures';

/** A loop was drawn while the area of a color (`key`) is open: it is added to the area or taken away from it. `stroke` is in the photo's pixels. */
function drawOnColorArea(key: string, stroke: DrawnPoint[], mode: DrawMode) {
  const image = currentImage.value as AbraImageLike | null;
  if (!image) return;
  const rings = combineArea(areaPoints(adjustments.value[key]), stroke, mode, { width: image.width(), height: image.height() });
  commitColorArea(key, encodeAreaRings(rings));
}

/**
 * Offers the area of the open color to draw (Color Splash's colors), through the same `AreaTarget` as every other control. A color
 * must be on before an area can attach to it.
 */
export function ColorAreaToolController() {
  const sectionKey = useObservable(activeSectionKey$, activeSectionKey.value);
  const edits = useObservable(adjustments$, adjustments.value);
  const focusedKey = useObservable(focusedControlKey$, focusedControlKey.value);
  const open = useObservable(openSliderState$, openSliderState.value);
  const focused = focusedKey ? leafControls(findSection(sectionKey).controls).find(control => control.key === focusedKey) : undefined;
  const slider = focused?.kind === 'slider' && focused.areaKeys ? focused : null;
  const index = slider && open.key === slider.key ? open.index : 0;
  const key = slider?.areaKeys?.[index] ?? null;
  const on = slider ? sliderValues(slider, edits[slider.key])[index] > (sliderSpecs(slider)[index]?.min ?? 0) : false;
  const area = key ? edits[key] : undefined;
  const target = useMemo(
    () =>
      key
        ? {
            key,
            canDraw: on,
            colorIndex: index,
            hint: {
              new: 'Draw the area for this color',
              add: 'Draw to add to this color',
              subtract: 'Draw to take away from this color',
            },
            onClear: () => commitColorArea(key, null),
            onDrawn: (stroke: DrawnPoint[], mode: DrawMode) => drawOnColorArea(key, stroke, mode),
          }
        : null,
    [key, on, index],
  );
  useAreaTarget('color-area', target);
  // The color's area is kept in the edits, so it is copied into the list of areas while the color is open.
  useEffect(() => {
    if (key) setArea(key, areaPoints(area) ?? []);
  }, [key, area]);
  return null;
}

const AREA_OUTLINE_COLORS = ['#ffffff', '#ffd54f', '#4fc3f7'];

/** Draws the area the open control offers on the photo, in the same space as the person outlines so they zoom and turn with it. */
export function AreaOutlines({ imageSize, previewSize }: { imageSize: { width: number; height: number }; previewSize: PreviewBox }) {
  const scale = useObservable(previewScaleSettled$, previewScaleValue.value);
  const hidden = useObservable(areaOutlinesHidden$, areaOutlinesHidden.value);
  const target = useObservable(areaTarget$, areaTarget.value);
  const drawn = useObservable(areas$, areas.value);
  const rings = target ? areaRings(drawn, target.key) : [];
  const shown = target && rings.length > 0 ? [{ index: target.colorIndex ?? 0, rings }] : [];
  if (hidden || shown.length === 0) return null;

  const fit = Math.min(previewSize.width / imageSize.width, previewSize.height / imageSize.height);
  const offsetX = (previewSize.width - imageSize.width * fit) / 2;
  const offsetY = (previewSize.height - imageSize.height * fit) / 2;
  // Divided by the zoom so the lines stay the same thickness on screen.
  const strokeWidth = 2 / scale;

  return (
    <Svg style={StyleSheet.absoluteFill} pointerEvents="none">
      {shown.map(({ index, rings }) => {
        // Every ring is one closed subpath; the even-odd fill makes a ring inside another a hole.
        const d = rings
          .map(ring =>
            ring
              .map(
                ([x, y], i) => `${i === 0 ? 'M' : 'L'} ${offsetX + x * imageSize.width * fit} ${offsetY + y * imageSize.height * fit}`,
              )
              .join(' ') + ' Z',
          )
          .join(' ');
        const color = AREA_OUTLINE_COLORS[index % AREA_OUTLINE_COLORS.length];
        return (
          <Path
            key={index}
            d={d}
            stroke={color}
            strokeWidth={strokeWidth}
            fill={color}
            fillOpacity={0.12}
            fillRule="evenodd"
            strokeLinejoin="round"
          />
        );
      })}
    </Svg>
  );
}
