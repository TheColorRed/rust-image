import { useTheme } from '@/src/lib/theme';
import { ArrowLeftIcon, CheckIcon, ImageMinusIcon, ImagesIcon, XIcon } from 'lucide-react-native';
import { useState } from 'react';
import { FlatList, Image, Modal, StyleSheet, Text, TouchableOpacity, useWindowDimensions, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';

/**
 * Select mode for a grid: it is on while anything is selected, and emptying the selection ends it. What starts it
 * (a tap or a long press) is up to the grid. Items are told apart by an id, such as a URI.
 */
export function useSelection() {
  const [selected, setSelected] = useState<string[]>([]);
  // The ids the preview pages through. Fixed when it opens, so unselecting a photo there does not drop its page.
  const [previewIds, setPreviewIds] = useState<string[] | null>(null);
  return {
    selected,
    selecting: selected.length > 0,
    toggle: (id: string) =>
      setSelected(current => (current.includes(id) ? current.filter(selectedId => selectedId !== id) : [...current, id])),
    /** Ends Select mode, and closes the preview with it since nothing is left to preview. */
    clear: () => {
      setSelected([]);
      setPreviewIds(null);
    },
    /** Selects every photo the preview pages through. */
    selectAllPreviewed: () => setSelected(previewIds ?? []),
    /** Unselects everything but leaves the preview open, so photos can be picked again. */
    unselectAll: () => setSelected([]),
    previewIds,
    openPreview: () => setPreviewIds(selected),
    closePreview: () => setPreviewIds(null),
  };
}

/**
 * The pill that floats at the bottom of a grid in Select mode, like the system photo picker's: close and the count on
 * the left, the action for the selection on the right. Place it last inside the grid's container, and leave
 * `SELECTION_BAR_CLEARANCE` of bottom padding in the list so the last row can scroll clear of it.
 */
export function SelectionBar({
  count,
  actionLabel,
  destructive = false,
  onCancel,
  onPreview,
  onAction,
}: {
  count: number;
  actionLabel: string;
  destructive?: boolean;
  onCancel: () => void;
  /** Shows a Preview button before the action. */
  onPreview?: () => void;
  onAction: () => void;
}) {
  const { colors } = useTheme();
  return (
    <View
      style={{
        position: 'absolute',
        left: 16,
        right: 16,
        bottom: 16,
        flexDirection: 'row',
        alignItems: 'center',
        justifyContent: 'space-between',
        paddingVertical: 8,
        paddingLeft: 8,
        paddingRight: 8,
        borderRadius: 32,
        borderWidth: 1,
        borderColor: colors.border,
        backgroundColor: colors.surface,
        elevation: 8,
        shadowColor: '#000',
        shadowOpacity: 0.3,
        shadowRadius: 8,
        shadowOffset: { width: 0, height: 2 },
      }}
    >
      <View style={{ flexDirection: 'row', alignItems: 'center', gap: 8 }}>
        <TouchableOpacity
          accessibilityRole="button"
          accessibilityLabel="Cancel selection"
          onPress={onCancel}
          style={{ width: 44, height: 44, alignItems: 'center', justifyContent: 'center' }}
        >
          <XIcon size={24} color={colors.textPrimary} />
        </TouchableOpacity>
        <Text style={{ color: colors.textPrimary, fontSize: 20 }}>{count}</Text>
      </View>
      <View style={{ flexDirection: 'row', alignItems: 'center', gap: 8 }}>
        {onPreview && (
          <TouchableOpacity
            accessibilityRole="button"
            onPress={onPreview}
            style={{ paddingHorizontal: 12, height: 48, justifyContent: 'center' }}
          >
            <Text style={{ color: colors.accent, fontSize: 16, fontWeight: '600' }}>Preview</Text>
          </TouchableOpacity>
        )}
        <TouchableOpacity
          accessibilityRole="button"
          onPress={onAction}
          style={{
            paddingHorizontal: 28,
            height: 48,
            alignItems: 'center',
            justifyContent: 'center',
            borderRadius: 24,
            backgroundColor: destructive ? colors.danger : colors.accent,
          }}
        >
          <Text style={{ color: '#fff', fontSize: 16, fontWeight: '600' }}>{actionLabel}</Text>
        </TouchableOpacity>
      </View>
    </View>
  );
}

/** Bottom padding a list needs while the floating bar is showing, so its last row is not covered. */
export const SELECTION_BAR_CLEARANCE = 88;

/** Dims a selected thumbnail and marks it with a check. Place it inside the thumbnail's `Pressable`. */
export function SelectedOverlay() {
  const { colors } = useTheme();
  return (
    <View
      style={{
        ...StyleSheet.absoluteFill,
        backgroundColor: 'rgba(0,0,0,0.4)',
        alignItems: 'flex-end',
        justifyContent: 'flex-end',
        padding: 6,
      }}
    >
      <View style={{ backgroundColor: colors.accent, borderRadius: 12, padding: 3 }}>
        <CheckIcon size={16} color="#fff" strokeWidth={3} />
      </View>
    </View>
  );
}

/**
 * Full-screen preview of the photos that were selected when it opened, one per page; swipe sideways when there is
 * more than one. The circle at the top left selects or unselects the photo on screen, and Done runs the bar's action.
 * Renders nothing until `selection.openPreview()` is called.
 */
export function SelectionPreview({
  selection,
  uriOf,
  doneLabel,
  destructive = false,
  onDone,
}: {
  selection: ReturnType<typeof useSelection>;
  /** The image to show for an id, or undefined if it is gone. */
  uriOf: (id: string) => string | undefined;
  doneLabel: string;
  destructive?: boolean;
  onDone: () => void;
}) {
  const { colors } = useTheme();
  const insets = useSafeAreaInsets();
  const { width } = useWindowDimensions();
  const [index, setIndex] = useState(0);
  const items = (selection.previewIds ?? []).flatMap(id => {
    const uri = uriOf(id);
    return uri === undefined ? [] : [{ id, uri }];
  });
  const current = items[index];
  const isSelected = current !== undefined && selection.selected.includes(current.id);
  const cornerButton = { position: 'absolute', left: 8, width: 48, height: 48, alignItems: 'center', justifyContent: 'center' } as const;

  return (
    <Modal
      visible={selection.previewIds !== null}
      animationType="fade"
      statusBarTranslucent
      onRequestClose={selection.closePreview}
      onShow={() => setIndex(0)}
    >
      <View style={{ flex: 1, backgroundColor: '#000' }}>
        <FlatList
          // A new key re-lays the pages out for a new width, such as after turning the phone.
          key={width}
          horizontal
          pagingEnabled
          showsHorizontalScrollIndicator={false}
          data={items}
          keyExtractor={item => item.id}
          getItemLayout={(_, itemIndex) => ({ length: width, offset: width * itemIndex, index: itemIndex })}
          onMomentumScrollEnd={event => setIndex(Math.round(event.nativeEvent.contentOffset.x / width))}
          renderItem={({ item }) => (
            <View style={{ width, justifyContent: 'center' }}>
              <Image source={{ uri: item.uri }} resizeMode="contain" style={{ width: '100%', height: '100%' }} />
            </View>
          )}
        />

        <TouchableOpacity
          accessibilityRole="button"
          accessibilityLabel="Back"
          onPress={selection.closePreview}
          style={{ ...cornerButton, top: insets.top + 8 }}
        >
          <ArrowLeftIcon size={28} color="#fff" />
        </TouchableOpacity>

        {items.length > 1 && (
          <Text style={{ position: 'absolute', top: insets.top + 20, alignSelf: 'center', color: '#fff', fontSize: 16 }}>
            {index + 1} / {items.length}
          </Text>
        )}

        <TouchableOpacity
          accessibilityRole="checkbox"
          accessibilityState={{ checked: isSelected }}
          accessibilityLabel="Selected"
          onPress={() => current && selection.toggle(current.id)}
          style={{ ...cornerButton, top: insets.top + 64 }}
        >
          {isSelected ? (
            <View
              style={{
                width: 32,
                height: 32,
                borderRadius: 16,
                backgroundColor: colors.accent,
                alignItems: 'center',
                justifyContent: 'center',
              }}
            >
              <CheckIcon size={20} color="#fff" strokeWidth={3} />
            </View>
          ) : (
            <View style={{ width: 32, height: 32, borderRadius: 16, borderWidth: 2, borderColor: '#fff' }} />
          )}
        </TouchableOpacity>

        <View
          style={{
            position: 'absolute',
            left: 16,
            right: 16,
            bottom: insets.bottom + 16,
            flexDirection: 'row',
            alignItems: 'center',
            justifyContent: 'space-between',
          }}
        >
          <TouchableOpacity
            accessibilityRole="button"
            onPress={selection.selecting ? selection.unselectAll : selection.selectAllPreviewed}
            style={{ flexDirection: 'row', alignItems: 'center', gap: 8, height: 48 }}
          >
            {selection.selecting ? (
              <ImageMinusIcon size={24} color={colors.accent} />
            ) : (
              <ImagesIcon size={24} color={colors.accent} />
            )}
            <Text style={{ color: colors.accent, fontSize: 17 }}>
              {selection.selecting ? `Unselect all (${selection.selected.length})` : `Select all (${items.length})`}
            </Text>
          </TouchableOpacity>
          <TouchableOpacity
            accessibilityRole="button"
            disabled={!selection.selecting}
            onPress={onDone}
            style={{
              opacity: selection.selecting ? 1 : 0.5,
              paddingHorizontal: 32,
              height: 48,
              alignItems: 'center',
              justifyContent: 'center',
              borderRadius: 24,
              backgroundColor: destructive ? colors.danger : colors.accent,
            }}
          >
            <Text style={{ color: '#fff', fontSize: 16, fontWeight: '600' }}>{doneLabel}</Text>
          </TouchableOpacity>
        </View>
      </View>
    </Modal>
  );
}
