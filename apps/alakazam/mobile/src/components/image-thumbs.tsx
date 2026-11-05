import { useTheme } from '@/src/lib/ui/theme';
import { memo, useMemo, useState } from 'react';
import { Image, ScrollView, StyleSheet, View } from 'react-native';
import type { PickedImage } from './file-picker';
import { ImagePreview } from './image-preview';
import { SelectableThumb, useSelection } from './selection';

type Props = {
  /** The pictures to show, in order. */
  files: PickedImage[];
  /** Lets the thumbnails be selected (press and hold one). The caller owns it so it can show the `SelectionBar` at the bottom of the screen. Without it they can only be opened. */
  selection?: ReturnType<typeof useSelection>;
  /** How many rows of thumbnails there are. The pictures fill the rows a column at a time and the grid scrolls left and right. `'all'` shows every picture, wrapped onto as many rows as it takes, with nothing to scroll. Defaults to 2. */
  rows?: number | 'all';
  /** What a tap on a thumbnail does, given the picture and its place in `files`. Without it a tap opens the picture full size, with the others to swipe through. */
  onPress?: (file: PickedImage, index: number) => void;
  style?: any;
};

const THUMB = 56;
const GAP = 6;

/**
 * Pictures as a grid of small thumbnails. Tapping one opens the pictures full size, and holding one selects it when a `selection` is given.
 * It only shows pictures, so it works beside a file picker or anywhere else a list of pictures is kept. The selection is made again on every
 * render of the screen, but its functions only ever set state, so it is drawn again only when what is selected changes.
 */
export const ImageThumbs = memo(
  function ImageThumbs({ files, selection, rows = 2, onPress, style }: Props) {
    const { colors } = useTheme();
    const styles = useMemo(
      () =>
        StyleSheet.create({
          all: { width: '100%', flexDirection: 'row', flexWrap: 'wrap', gap: GAP },
          thumb: { width: THUMB, height: THUMB, borderRadius: 6, backgroundColor: colors.placeholder },
        }),
      [colors],
    );
    const unusedSelection = useSelection();
    const [previewIndex, setPreviewIndex] = useState<number | null>(null);
    if (files.length === 0) return null;
    // A fixed height makes the pictures wrap into columns, which is how the rows stay the same number and the grid grows to the right.
    const shownRows = rows === 'all' ? files.length : Math.max(1, Math.min(Math.round(rows), files.length));
    const grid =
      rows === 'all'
        ? styles.all
        : {
            flexDirection: 'column' as const,
            flexWrap: 'wrap' as const,
            gap: GAP,
            height: shownRows * THUMB + (shownRows - 1) * GAP,
          };
    const thumbs = (
      <>
        {files.map((file, index) => (
          <SelectableThumb
            key={`${file.uri}-${index}`}
            id={file.uri}
            selection={selection ?? unusedSelection}
            selectable={selection !== undefined}
            onOpen={() => (onPress ? onPress(file, index) : setPreviewIndex(index))}
            accessibilityLabel={`Open ${file.name ?? `image ${index + 1}`}`}
          >
            <Image source={{ uri: file.uri }} style={styles.thumb} resizeMode="cover" />
          </SelectableThumb>
        ))}
      </>
    );
    return (
      <View style={style}>
        {rows === 'all' ? (
          <View style={grid}>{thumbs}</View>
        ) : (
          <ScrollView
            horizontal
            showsHorizontalScrollIndicator={false}
            nestedScrollEnabled
            contentContainerStyle={grid}
          >
            {thumbs}
          </ScrollView>
        )}
        <ImagePreview
          images={files.map((file, index) => ({ id: `${file.uri}-${index}`, uri: file.uri }))}
          visible={previewIndex != null}
          initialIndex={previewIndex ?? 0}
          onClose={() => setPreviewIndex(null)}
        />
      </View>
    );
  },
  (before, after) =>
    before.files === after.files &&
    before.rows === after.rows &&
    before.onPress === after.onPress &&
    before.style === after.style &&
    before.selection?.selected === after.selection?.selected &&
    before.selection?.previewIds === after.selection?.previewIds,
);
