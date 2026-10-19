import { faceBoxes, faceBoxes$ } from '@/src/state/analysis';
import { previewScaleSettled$, previewScaleValue } from '@/src/state/gestures';
import { personSelection, selectionNeeded } from '@/src/state/person-selection';
import { StyleSheet, Text, View } from 'react-native';
import { useObservable } from 'react-rx';
import Svg, { Path } from 'react-native-svg';

const PERSON_OUTLINE_COLORS = [
  '#FF4FA3',
  '#FF453A',
  '#34C759',
  '#0A84FF',
  '#FFD60A',
  '#FF9F0A',
  '#BF5AF2',
  '#64D2FF',
  '#A8E66B',
  '#FF6B6B',
] as const;

type PersonSelectionState = typeof personSelection.value;
type Size = { width: number; height: number };

/** Whether the person outlines and their notice are on show. */
export const personOverlayShown = (state: PersonSelectionState, skinControlFocused: boolean, outlinesVisible: boolean) =>
  skinControlFocused && state.focused && outlinesVisible && selectionNeeded(state);

const noticeText = (state: PersonSelectionState) => {
  if (state.error) return state.error;
  if (state.loading) return 'Finding bodies…';
  if (state.people.length === 0) return 'No bodies detected; skin effects still work';
  if (state.selectedId === null) return 'Tap a person to target skin effects';
  return `Body ${state.selectedId + 1} selected`;
};

/** The notice about the people found, kept outside the zoomed view so it keeps its size and stays centered. */
export function PersonDetectionNotice({ state }: { state: PersonSelectionState }) {
  return (
    <View
      pointerEvents="none"
      style={{
        position: 'absolute',
        top: 12,
        alignSelf: 'center',
        paddingHorizontal: 10,
        paddingVertical: 5,
        borderRadius: 14,
        backgroundColor: 'rgba(0,0,0,0.68)',
      }}
    >
      <Text style={{ color: '#fff', fontSize: 12, fontWeight: '600' }}>{noticeText(state)}</Text>
    </View>
  );
}

export function PersonDetectionOverlay({
  people,
  selectedId,
  imageSize,
  previewSize,
}: {
  people: PersonSelectionState['people'];
  selectedId: number | null;
  imageSize: Size;
  previewSize: Size;
}) {
  const faces = useObservable(faceBoxes$, faceBoxes.value);
  const scale = useObservable(previewScaleSettled$, previewScaleValue.value);
  const fit = Math.min(previewSize.width / imageSize.width, previewSize.height / imageSize.height);
  const offsetX = (previewSize.width - imageSize.width * fit) / 2;
  const offsetY = (previewSize.height - imageSize.height * fit) / 2;

  return (
    <View pointerEvents="none" style={StyleSheet.absoluteFill}>
      {people.map(person => {
        const selected = person.id === selectedId;
        const face = faces?.[person.id];
        const color = PERSON_OUTLINE_COLORS[person.id % PERSON_OUTLINE_COLORS.length];
        const opacity = selectedId === null || selected ? 1 : 0.55;
        // The overlay is zoomed with the photo, so divide by the zoom to keep the lines the same thickness on screen.
        const strokeWidth = (selected ? 3 : 2) / scale;
        // A face is framed by a short arc at each side of a tall oval, with the top and bottom left open, and never filled in.
        if (face) {
          const cx = offsetX + (face.x + face.width / 2) * fit;
          const cy = offsetY + (face.y + face.height / 2) * fit;
          const rx = face.width * fit * 0.47;
          const ry = face.height * fit * 0.62;
          const point = (degrees: number) => {
            const radians = (degrees * Math.PI) / 180;
            return `${cx + rx * Math.cos(radians)} ${cy + ry * Math.sin(radians)}`;
          };
          const arc = (from: number, to: number) => `M ${point(from)} A ${rx} ${ry} 0 0 1 ${point(to)}`;
          return (
            <Svg key={person.id} style={StyleSheet.absoluteFill} opacity={opacity}>
              <Path d={arc(-28, 28)} stroke={color} strokeWidth={strokeWidth} strokeLinecap="round" fill="none" />
              <Path d={arc(152, 208)} stroke={color} strokeWidth={strokeWidth} strokeLinecap="round" fill="none" />
            </Svg>
          );
        }
        // Until the face is found, the frame is the box around the body.
        return (
          <View
            key={person.id}
            style={{
              position: 'absolute',
              left: offsetX + person.x * fit,
              top: offsetY + person.y * fit,
              width: person.width * fit,
              height: person.height * fit,
              opacity,
              borderWidth: strokeWidth,
              borderColor: color,
            }}
          />
        );
      })}
    </View>
  );
}
