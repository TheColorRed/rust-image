import { AppText } from '@/src/components/app-text';
import { Card, CardHeader } from '@/src/components/card';
import { MagicalSlogan } from '@/src/components/magical-slogan';
import {
  deleteInstalledFont,
  downloadFont,
  previewUri,
  refreshGoogleFonts,
  syncInstalledFonts,
} from '@/src/lib/fonts/google-fonts';
import { useCommonStyles } from '@/src/lib/ui/styles';
import { useTheme } from '@/src/lib/ui/theme';
import { errorMessage } from '@/src/lib/utils/error-message';
import { formatBytes } from '@/src/lib/utils/format-bytes';
import { googleFonts, googleFonts$, installedFonts, installedFonts$, type GoogleFont } from '@/src/state/fonts';
import RefreshCw from 'lucide-react-native/icons/refresh-cw';
import X from 'lucide-react-native/icons/x';
import { useCallback, useDeferredValue, useEffect, useMemo, useState } from 'react';
import { ActivityIndicator, Alert, FlatList, Image, Pressable, ScrollView, StyleSheet, Text, TextInput, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { useObservable } from 'react-rx';

/** The family's name in its own font. The picture is asked for when the row mounts, which the list does only near the screen. */
function FontName({ font, textStyle, imageStyle }: { font: GoogleFont; textStyle: object; imageStyle: object }) {
  const { colors } = useTheme();
  const [uri, setUri] = useState<string | null>(null);
  useEffect(() => {
    let current = true;
    // A row that is gone again within this time (typing, a fast scroll) never asks, so nothing queues up behind rows nobody sees.
    const wait = setTimeout(() => {
      previewUri(font, colors.textPrimary)
        .then(made => current && setUri(made))
        .catch(() => {});
    }, 250);
    return () => {
      current = false;
      clearTimeout(wait);
    };
    // The family names the picture; a `font` object made again on every render must not ask again.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [font.family, colors.textPrimary]);
  return uri ? (
    <Image accessibilityLabel={font.family} source={{ uri }} resizeMode="contain" style={imageStyle} />
  ) : (
    <Text style={textStyle}>{font.family}</Text>
  );
}

/**
 * Browse the fonts on this device, and download more from Google Fonts. It is the Fonts screen, and with `onPick` it is also the chooser a
 * plugin's `fontPicker` opens: then tapping a font on this device (or one just downloaded) picks it.
 */
export function FontBrowser({
  onPick,
  selected,
  onClose,
}: {
  /** Called with the family tapped. Without it the list only manages fonts. */
  onPick?: (family: string) => void;
  /** The family to mark as chosen. */
  selected?: string;
  /** Shows a Close button, for a browser opened over another screen. */
  onClose?: () => void;
}) {
  const { colors } = useTheme();
  const commonStyles = useCommonStyles();
  const insets = useSafeAreaInsets();
  const available = useObservable(googleFonts$, googleFonts.value);
  const installed = useObservable(installedFonts$, installedFonts.value);
  const [search, setSearch] = useState('');
  /** The list follows this, so typing stays responsive while a long list is filtered. */
  const searched = useDeferredValue(search);
  /** The category chip chosen, or null for all fonts. */
  const [category, setCategory] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(available.length === 0);
  const [error, setError] = useState<string | null>(null);
  /** How far each family being downloaded has got, from 0 to 1. */
  const [progress, setProgress] = useState<Record<string, number>>({});
  const styles = useMemo(
    () =>
      StyleSheet.create({
        container: { flex: 1, backgroundColor: colors.background, paddingTop: onClose ? insets.top + 12 : 24, paddingBottom: onClose ? insets.bottom : 0 },
        close: { paddingHorizontal: 8, paddingVertical: 8 },
        chosen: { backgroundColor: colors.chip },
        searchBox: {
          flexDirection: 'row',
          alignItems: 'center',
          marginHorizontal: 16,
          marginBottom: 8,
          paddingLeft: 14,
          borderRadius: 10,
          borderWidth: 1,
          borderColor: colors.border,
        },
        search: { flex: 1, paddingVertical: 10, color: colors.textPrimary, fontSize: 16 },
        chips: { flexGrow: 0, marginBottom: 8 },
        chipsContent: { paddingHorizontal: 16, gap: 8 },
        chip: { paddingHorizontal: 14, paddingVertical: 6, borderRadius: 16, borderWidth: 1, borderColor: colors.border },
        chipOn: { backgroundColor: colors.chip, borderColor: colors.accent },
        chipText: { fontSize: 14, color: colors.textPrimary, textTransform: 'capitalize' },
        clear: { width: 40, height: 40, alignItems: 'center', justifyContent: 'center' },
        cards: { flex: 1, paddingHorizontal: 16, paddingBottom: 12, gap: 12 },
        installedList: { maxHeight: 240 },
        googleCard: { flex: 1 },
        section: { paddingHorizontal: 16, paddingTop: 16, paddingBottom: 6, fontSize: 15, fontWeight: '600', color: colors.textSecondary },
        row: {
          flexDirection: 'row',
          alignItems: 'center',
          gap: 12,
          paddingHorizontal: 16,
          height: 72,
          borderTopWidth: 1,
          borderTopColor: colors.border,
        },
        rowText: { flex: 1 },
        rowTitle: { fontSize: 16, color: colors.textPrimary },
        rowPreview: { height: 28, width: '100%', alignSelf: 'flex-start' },
        rowSize: { marginTop: 2, fontSize: 13, color: colors.textSecondary },
        button: { minWidth: 96, paddingVertical: 8, borderRadius: 8, alignItems: 'center' },
        download: { backgroundColor: colors.accent },
        remove: { borderWidth: 1, borderColor: colors.danger },
        busy: { borderWidth: 1, borderColor: colors.accent },
        buttonText: { fontSize: 14, fontWeight: '600' },
        refresh: { width: 40, height: 40, alignItems: 'center', justifyContent: 'center', borderRadius: 20 },
        error: { paddingHorizontal: 16, paddingBottom: 8, fontSize: 13, color: colors.danger },
      }),
    [colors, onClose, insets.top, insets.bottom],
  );

  const reload = useCallback(async () => {
    setError(null);
    setRefreshing(true);
    syncInstalledFonts();
    try {
      await refreshGoogleFonts();
    } catch (e) {
      setError(errorMessage(e));
    }
    setRefreshing(false);
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  const download = async (font: GoogleFont) => {
    setError(null);
    setProgress(current => ({ ...current, [font.family]: 0 }));
    try {
      await downloadFont(font, { update: share => setProgress(current => ({ ...current, [font.family]: share })) });
    } catch (e) {
      setError(errorMessage(e));
    }
    setProgress(({ [font.family]: _done, ...rest }) => rest);
  };

  const remove = (family: string) =>
    Alert.alert('Delete font?', `Delete ${family} from this device?`, [
      { text: 'Cancel', style: 'cancel' },
      {
        text: 'Delete',
        style: 'destructive',
        onPress: () => {
          try {
            deleteInstalledFont(family);
          } catch (e) {
            setError(errorMessage(e));
          }
        },
      },
    ]);

  const have = useMemo(() => new Set(installed.map(font => font.family)), [installed]);
  const categories = useMemo(() => [...new Set(available.map(font => font.category).filter(Boolean))].sort(), [available]);
  const matches = useMemo(() => {
    const words = searched.trim().toLowerCase().split(/\s+/).filter(Boolean);
    return available.filter(
      font => (category === null || font.category === category) && words.every(word => font.family.toLowerCase().includes(word)),
    );
  }, [available, searched, category]);

  const renderFont = ({ item }: { item: GoogleFont }) => {
    const downloaded = have.has(item.family);
    const share = progress[item.family];
    const styleCount = Object.keys(item.files).length;
    return (
      <View style={[styles.row, onPick && selected === item.family && styles.chosen]}>
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={onPick && downloaded ? `Use ${item.family}` : item.family}
          disabled={!onPick || !downloaded}
          onPress={() => onPick?.(item.family)}
          style={styles.rowText}
        >
          <FontName font={item} textStyle={styles.rowTitle} imageStyle={styles.rowPreview} />
          <Text style={styles.rowSize}>
            {item.category.replace('-', ' ')} · {styleCount} {styleCount === 1 ? 'style' : 'styles'}
          </Text>
        </Pressable>
        {share !== undefined ? (
          <View style={[styles.button, styles.busy]}>
            <Text style={[styles.buttonText, { color: colors.textPrimary }]}>{Math.round(share * 100)}%</Text>
          </View>
        ) : downloaded ? (
          <Pressable accessibilityRole="button" accessibilityLabel={`Delete ${item.family}`} onPress={() => remove(item.family)} style={[styles.button, styles.remove]}>
            <Text style={[styles.buttonText, { color: colors.danger }]}>Delete</Text>
          </Pressable>
        ) : (
          <Pressable accessibilityRole="button" accessibilityLabel={`Download ${item.family}`} onPress={() => void download(item)} style={[styles.button, styles.download]}>
            <Text style={[styles.buttonText, { color: '#fff' }]}>Download</Text>
          </Pressable>
        )}
      </View>
    );
  };

  return (
    <View style={styles.container}>
      <View style={[commonStyles.header, { alignItems: 'center' }]}>
        <AppText size="heading" bold>
          Fonts
        </AppText>
        {onClose && (
          <Pressable accessibilityRole="button" accessibilityLabel="Close fonts" hitSlop={8} onPress={onClose} style={styles.close}>
            <Text style={{ color: colors.accent, fontWeight: '600', fontSize: 16 }}>Close</Text>
          </Pressable>
        )}
        <Pressable accessibilityRole="button" accessibilityLabel="Refresh fonts" disabled={refreshing} hitSlop={8} onPress={() => void reload()} style={styles.refresh}>
          {refreshing ? <ActivityIndicator color={colors.accent} /> : <RefreshCw size={22} color={colors.accent} strokeWidth={1.8} />}
        </Pressable>
      </View>
      <View style={styles.searchBox}>
        <TextInput
          accessibilityLabel="Search fonts"
          autoCapitalize="none"
          autoCorrect={false}
          onChangeText={setSearch}
          placeholder="Search fonts"
          placeholderTextColor={colors.textSecondary}
          style={styles.search}
          value={search}
        />
        {search !== '' && (
          <Pressable accessibilityRole="button" accessibilityLabel="Clear search" hitSlop={8} onPress={() => setSearch('')} style={styles.clear}>
            <X size={20} color={colors.textSecondary} strokeWidth={1.8} />
          </Pressable>
        )}
      </View>
      {categories.length > 0 && (
        <ScrollView horizontal showsHorizontalScrollIndicator={false} keyboardShouldPersistTaps="handled" style={styles.chips} contentContainerStyle={styles.chipsContent}>
          {[null, ...categories].map(name => (
            <Pressable
              key={name ?? 'all'}
              accessibilityRole="button"
              accessibilityState={{ selected: category === name }}
              accessibilityLabel={name ? `Show ${name.replace('-', ' ')} fonts` : 'Show all fonts'}
              onPress={() => setCategory(name)}
              style={[styles.chip, category === name && styles.chipOn]}
            >
              <Text style={styles.chipText}>{name ? name.replace('-', ' ') : 'All'}</Text>
            </Pressable>
          ))}
        </ScrollView>
      )}
      {error && <Text style={styles.error}>{error}</Text>}
      {refreshing && available.length === 0 ? (
        <View style={{ minHeight: 240, justifyContent: 'center', alignItems: 'center' }}>
          <MagicalSlogan fontSize={32} subText="Loading fonts..." />
        </View>
      ) : (
        <View style={styles.cards}>
          {installed.length > 0 && searched.trim() === '' && (
            <Card flush>
              <CardHeader title="On this device" />
              <ScrollView style={styles.installedList} keyboardShouldPersistTaps="handled">
                {installed.map(font => (
                  <View key={font.family} style={[styles.row, onPick && selected === font.family && styles.chosen]}>
                    <Pressable
                      accessibilityRole="button"
                      accessibilityLabel={onPick ? `Use ${font.family}` : font.family}
                      disabled={!onPick}
                      onPress={() => onPick?.(font.family)}
                      style={styles.rowText}
                    >
                      <FontName
                        font={{ family: font.family, category: '', files: {} }}
                        textStyle={styles.rowTitle}
                        imageStyle={styles.rowPreview}
                      />
                      <Text style={styles.rowSize}>
                        {font.files.length} {font.files.length === 1 ? 'style' : 'styles'} · {formatBytes(Number(font.bytes))}
                      </Text>
                    </Pressable>
                    <Pressable accessibilityRole="button" accessibilityLabel={`Delete ${font.family}`} onPress={() => remove(font.family)} style={[styles.button, styles.remove]}>
                      <Text style={[styles.buttonText, { color: colors.danger }]}>Delete</Text>
                    </Pressable>
                  </View>
                ))}
              </ScrollView>
            </Card>
          )}
          <Card flush style={styles.googleCard}>
            <CardHeader title="Google Fonts" />
            <FlatList
              data={matches}
              initialNumToRender={10}
              maxToRenderPerBatch={6}
              windowSize={5}
              keyboardShouldPersistTaps="handled"
              keyExtractor={font => font.family}
              ListEmptyComponent={error ? undefined : <Text style={styles.section}>No fonts found</Text>}
              renderItem={renderFont}
            />
          </Card>
        </View>
      )}
    </View>
  );
}
