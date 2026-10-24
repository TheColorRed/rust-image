import ArrowLeftIcon from 'lucide-react-native/icons/arrow-left';
import { useTheme } from '@/src/lib/ui/theme';
import { useRef, useState, type ReactNode } from 'react';
import { FlatList, Image, Modal, Text, TouchableOpacity, useWindowDimensions, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';

export interface PreviewImage {
  id: string;
  uri: string;
}

/** Full-screen, swipeable image viewer. Optional controls are rendered over the image. */
export function ImagePreview({
  images,
  visible,
  initialIndex = 0,
  onClose,
  renderControls,
}: {
  images: PreviewImage[];
  visible: boolean;
  initialIndex?: number;
  onClose: () => void;
  renderControls?: (state: { index: number; current?: PreviewImage; count: number }) => ReactNode;
}) {
  const { colors } = useTheme();
  const insets = useSafeAreaInsets();
  const { width } = useWindowDimensions();
  const [index, setIndex] = useState(initialIndex);
  const list = useRef<FlatList<PreviewImage>>(null);
  const current = images[index];
  const cornerButton = { position: 'absolute', left: 8, width: 48, height: 48, alignItems: 'center', justifyContent: 'center' } as const;

  return (
    <Modal
      visible={visible}
      animationType="fade"
      statusBarTranslucent
      onRequestClose={onClose}
      onShow={() => {
        setIndex(initialIndex);
        if (images.length > 0) list.current?.scrollToIndex({ index: initialIndex, animated: false });
      }}
    >
      <View style={{ flex: 1, backgroundColor: '#000' }}>
        <FlatList
          ref={list}
          key={width}
          horizontal
          pagingEnabled
          showsHorizontalScrollIndicator={false}
          data={images}
          keyExtractor={item => item.id}
          getItemLayout={(_, itemIndex) => ({ length: width, offset: width * itemIndex, index: itemIndex })}
          initialScrollIndex={initialIndex}
          onMomentumScrollEnd={event => setIndex(Math.round(event.nativeEvent.contentOffset.x / width))}
          renderItem={({ item }) => (
            <View style={{ width, justifyContent: 'center' }}>
              <Image source={{ uri: item.uri }} resizeMode="contain" style={{ width: '100%', height: '100%' }} />
            </View>
          )}
        />
        <TouchableOpacity accessibilityRole="button" accessibilityLabel="Back" onPress={onClose} style={{ ...cornerButton, top: insets.top + 8 }}>
          <ArrowLeftIcon size={28} color="#fff" />
        </TouchableOpacity>
        {images.length > 1 && (
          <View
            style={{
              position: 'absolute',
              top: insets.top + 12,
              alignSelf: 'center',
              minHeight: 34,
              alignItems: 'center',
              justifyContent: 'center',
              paddingHorizontal: 12,
              borderRadius: 17,
              backgroundColor: colors.overlayButton,
            }}
          >
            <Text style={{ color: '#fff', fontSize: 14, fontWeight: '600' }}>{index + 1} / {images.length}</Text>
          </View>
        )}
        {renderControls?.({ index, current, count: images.length })}
      </View>
    </Modal>
  );
}
