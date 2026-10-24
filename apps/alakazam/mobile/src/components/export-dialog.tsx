import { ImagePreview } from '@/src/components/image-preview';
import { MagicalSlogan } from '@/src/components/magical-slogan';
import { ProgressBar } from '@/src/components/progress-bar';
import { useTheme } from '@/src/lib/ui/theme';
import { errorMessage } from '@/src/lib/utils/error-message';
import type { ExportFormat } from '@/src/state/settings';
import { EXPORT_FORMATS, exportFormat, exportFormat$, exportQuality, exportQuality$ } from '@/src/state/settings';
import RNSlider from '@react-native-community/slider';
import XIcon from 'lucide-react-native/icons/x';
import { useEffect, useState } from 'react';
import {
  ActivityIndicator,
  Alert,
  FlatList,
  Image,
  Modal,
  Pressable,
  Text,
  TextInput,
  TouchableOpacity,
  View,
} from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { useObservable } from 'react-rx';

export interface ExportDialogItem {
  id: string;
  uri: string;
  name: string;
}

export function ExportDialog({
  items,
  visible,
  onClose,
  onSaveImage,
}: {
  items: ExportDialogItem[];
  visible: boolean;
  onClose: () => void;
  onSaveImage: (item: ExportDialogItem, fileName: string, format: ExportFormat, quality: number) => Promise<void>;
}) {
  const { colors } = useTheme();
  const insets = useSafeAreaInsets();
  const [names, setNames] = useState<Record<string, string>>({});
  const [exporting, setExporting] = useState(false);
  const [savedCount, setSavedCount] = useState(0);
  const [previewId, setPreviewId] = useState<string | null>(null);
  const [excludedIds, setExcludedIds] = useState<string[]>([]);
  const format = useObservable(exportFormat$, exportFormat.value);
  const quality = useObservable(exportQuality$, exportQuality.value);
  const qualityValue = quality;

  useEffect(() => {
    if (visible) {
      setNames(Object.fromEntries(items.map(item => [item.id, item.name.replace(/\.[^.]+$/, '') || 'image'])));
      setExcludedIds([]);
    }
  }, [visible, items]);

  const activeItems = items.filter(item => !excludedIds.includes(item.id));

  const exportImages = async () => {
    exportFormat.next('webp');
    exportQuality.next(85);
    setSavedCount(0);
    setExporting(true);
    let saved = 0;
    try {
      for (const item of activeItems) {
        const name = (names[item.id] ?? item.name.replace(/\.[^.]+$/, '')).trim();
        if (!name) throw new Error(`Enter a filename for ${item.name}.`);
        await onSaveImage(item, `${name}.${format}`, format, qualityValue);
        saved += 1;
        setSavedCount(saved);
      }
      // Keep the 100% state visible briefly before closing the dialog.
      await new Promise(resolve => setTimeout(resolve, 350));
      onClose();
      Alert.alert(
        'Export complete',
        `${saved} image${saved === 1 ? '' : 's'} saved as ${format.toUpperCase()} to Pictures/Alakazam.`,
      );
    } catch (error) {
      Alert.alert('Could not export the images', `${saved} of ${activeItems.length} saved. ${errorMessage(error)}`);
    } finally {
      setExporting(false);
    }
  };

  const cancelExport = () => {
    exportFormat.next('webp');
    exportQuality.next(85);
    onClose();
  };

  const previewIndex = Math.max(
    0,
    activeItems.findIndex(item => item.id === previewId),
  );
  return (
    <>
      <Modal visible={visible} animationType="slide" onRequestClose={cancelExport}>
        <View
          style={{
            flex: 1,
            backgroundColor: colors.background,
            paddingTop: insets.top + 12,
            paddingBottom: insets.bottom + 12,
          }}
        >
          <Text style={{ color: colors.textPrimary, fontSize: 22, fontWeight: '700', paddingHorizontal: 20 }}>
            Export images
          </Text>
          <Text style={{ color: colors.textMuted, paddingHorizontal: 20, marginTop: 6, marginBottom: 12 }}>
            Rename any image or keep its default name.
          </Text>
          {exporting && (
            <View style={{ marginHorizontal: 20, marginBottom: 12 }}>
              <ProgressBar
                label="Saving images"
                detail={`${savedCount}/${activeItems.length} · ${activeItems.length > 0 ? Math.round((savedCount / activeItems.length) * 100) : 0}%`}
                progress={activeItems.length > 0 ? savedCount / activeItems.length : 0}
              />
            </View>
          )}
          <View
            style={{
              marginHorizontal: 16,
              marginBottom: 12,
              padding: 14,
              gap: 14,
              borderRadius: 16,
              borderWidth: 1,
              borderColor: colors.border,
              backgroundColor: colors.surface,
            }}
          >
            <View style={{ gap: 8 }}>
              <Text style={{ color: colors.textPrimary, fontSize: 15, fontWeight: '600' }}>Export format</Text>
              <View style={{ flexDirection: 'row', backgroundColor: colors.background, borderRadius: 10, padding: 3 }}>
                {EXPORT_FORMATS.map(option => (
                  <TouchableOpacity
                    key={option.value}
                    accessibilityRole="radio"
                    accessibilityState={{ selected: format === option.value }}
                    disabled={exporting}
                    onPress={() => exportFormat.next(option.value)}
                    style={{
                      flex: 1,
                      paddingVertical: 8,
                      borderRadius: 8,
                      alignItems: 'center',
                      backgroundColor: format === option.value ? colors.accent : 'transparent',
                    }}
                  >
                    <Text
                      style={{
                        color: format === option.value ? '#fff' : colors.textSecondary,
                        fontSize: 14,
                        fontWeight: '600',
                      }}
                    >
                      {option.label}
                    </Text>
                  </TouchableOpacity>
                ))}
              </View>
            </View>
            {format !== 'png' && (
              <View style={{ gap: 4 }}>
                <View style={{ flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center' }}>
                  <Text style={{ color: colors.textPrimary, fontSize: 15, fontWeight: '600' }}>Export quality</Text>
                  <Text style={{ color: colors.accent, fontSize: 14, fontWeight: '600' }}>{qualityValue}%</Text>
                </View>
                <RNSlider
                  accessibilityLabel="Export quality"
                  minimumValue={0}
                  maximumValue={100}
                  step={1}
                  value={qualityValue}
                  disabled={exporting}
                  minimumTrackTintColor={colors.accent}
                  maximumTrackTintColor={colors.chip}
                  thumbTintColor={colors.accent}
                  onValueChange={value => exportQuality.next(Math.round(value))}
                  style={{ width: '100%', height: 40 }}
                />
              </View>
            )}
          </View>
          <FlatList
            data={activeItems}
            keyExtractor={item => item.id}
            contentContainerStyle={{ paddingHorizontal: 16, gap: 10 }}
            renderItem={({ item }) => (
              <View
                style={{
                  flexDirection: 'row',
                  alignItems: 'center',
                  gap: 12,
                  padding: 10,
                  borderRadius: 12,
                  backgroundColor: colors.surface,
                }}
              >
                <Pressable
                  accessibilityRole="button"
                  accessibilityLabel={`Preview ${item.name}`}
                  onPress={() => setPreviewId(item.id)}
                >
                  <Image
                    source={{ uri: item.uri }}
                    style={{ width: 64, height: 64, borderRadius: 8, backgroundColor: colors.placeholder }}
                  />
                </Pressable>
                <TextInput
                  accessibilityLabel={`Filename for ${item.name}`}
                  value={names[item.id] ?? item.name.replace(/\.[^.]+$/, '')}
                  onChangeText={name => setNames(current => ({ ...current, [item.id]: name }))}
                  editable={!exporting}
                  selectTextOnFocus
                  autoCapitalize="none"
                  style={{
                    flex: 1,
                    color: colors.textPrimary,
                    borderBottomWidth: 1,
                    borderBottomColor: colors.border,
                    paddingVertical: 8,
                  }}
                />
                <Text style={{ color: colors.textMuted }}>.{format}</Text>
                <TouchableOpacity
                  accessibilityRole="button"
                  accessibilityLabel={`Remove ${item.name} from export`}
                  disabled={exporting}
                  onPress={() => {
                    setExcludedIds(current => [...current, item.id]);
                    if (previewId === item.id) setPreviewId(null);
                  }}
                  style={{ width: 36, height: 36, alignItems: 'center', justifyContent: 'center' }}
                >
                  <XIcon size={20} color={colors.textSecondary} />
                </TouchableOpacity>
              </View>
            )}
            ListEmptyComponent={
              <View
                style={{
                  flex: 1,
                  minHeight: 180,
                  alignItems: 'center',
                  justifyContent: 'center',
                  paddingHorizontal: 24,
                }}
              >
                <MagicalSlogan fontSize={40} subText="No images selected for export." />
              </View>
            }
          />
          <View
            style={{
              flexDirection: 'row',
              justifyContent: 'flex-end',
              alignItems: 'center',
              gap: 16,
              paddingHorizontal: 20,
              paddingTop: 12,
            }}
          >
            <TouchableOpacity disabled={exporting} onPress={cancelExport} style={{ padding: 12 }}>
              <Text style={{ color: colors.textSecondary, fontSize: 16 }}>Cancel</Text>
            </TouchableOpacity>
            <TouchableOpacity
              disabled={exporting || activeItems.length === 0}
              onPress={exportImages}
              style={{
                minWidth: 112,
                alignItems: 'center',
                paddingHorizontal: 20,
                paddingVertical: 12,
                borderRadius: 24,
                backgroundColor: colors.accent,
                opacity: activeItems.length === 0 ? 0.5 : 1,
              }}
            >
              {exporting ? (
                <ActivityIndicator color="#fff" />
              ) : (
                <Text style={{ color: '#fff', fontSize: 16, fontWeight: '600' }}>Save {activeItems.length}</Text>
              )}
            </TouchableOpacity>
          </View>
        </View>
      </Modal>
      <ImagePreview
        images={activeItems.map(({ id, uri }) => ({ id, uri }))}
        visible={previewId !== null}
        initialIndex={previewIndex}
        onClose={() => setPreviewId(null)}
      />
    </>
  );
}
