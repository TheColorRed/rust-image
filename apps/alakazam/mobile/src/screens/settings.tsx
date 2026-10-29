import { AppText } from '@/src/components/app-text';
import { useCommonStyles } from '@/src/lib/ui/styles';
import { useTheme } from '@/src/lib/ui/theme';
import {
  EXPORT_FORMATS,
  EXPORT_QUALITIES,
  WORKER_STACK_CHOICES,
  exportFormat,
  exportFormat$,
  exportQuality,
  exportQuality$,
  developerModeEnabled,
  developerModeEnabled$,
  workerStackInEffect,
  workerStackInEffect$,
  workerStackMegabytes,
  workerStackMegabytes$,
} from '@/src/state/settings';
import { clearToolbarUsage } from '@/src/state/toolbar-usage';
import CircleQuestionMark from 'lucide-react-native/icons/circle-question-mark';
import { Fragment, useMemo, useRef, useState, type ElementRef } from 'react';
import { useObservable } from 'react-rx';
import { Alert, Modal, NativeModules, Pressable, ScrollView, StyleSheet, Switch, Text, View } from 'react-native';

/** The header over each card, by the name settings give their `card`. */
const CARD_TITLES: Record<string, string> = {
  appearance: 'Display',
  export: 'Export',
  performance: 'Performance',
  toolbar: 'Toolbar',
  developer: 'Developer',
};

export function SettingsScreen() {
  const { colors, preference, setPreference } = useTheme();
  const commonStyles = useCommonStyles();
  const format = useObservable(exportFormat$, exportFormat.value);
  const quality = useObservable(exportQuality$, exportQuality.value);
  const stack = useObservable(workerStackMegabytes$, workerStackMegabytes.value);
  const stackInEffect = useObservable(workerStackInEffect$, workerStackInEffect.value);
  const developerMode = useObservable(developerModeEnabled$, developerModeEnabled.value);
  const iconRefs = useRef<Record<string, ElementRef<typeof Pressable> | null>>({});
  /** The setting whose tooltip is open and the window point just under its icon, or null. */
  const [tip, setTip] = useState<{ key: string; x: number; y: number } | null>(null);
  const toggleDeveloperMode = async (enabled: boolean) => {
    if (!enabled) {
      try {
        await NativeModules.MediaFile?.stopPluginDevServer?.();
      } catch (error) {
        Alert.alert('Could not stop the development server', error instanceof Error ? error.message : String(error));
        return;
      }
    }
    developerModeEnabled.next(enabled);
  };
  const styles = useMemo(
    () =>
      StyleSheet.create({
        container: { flex: 1, backgroundColor: colors.background, paddingTop: 24 },
        // Each setting is a card of its own.
        section: {
          marginHorizontal: 16,
          marginBottom: 16,
          padding: 16,
          gap: 20,
          borderRadius: 16,
          borderWidth: 1,
          borderColor: colors.border,
          backgroundColor: colors.surface,
        },
        cards: { paddingBottom: 24 },
        setting: { gap: 12 },
        toolbarHistoryRow: { flexDirection: 'row', alignItems: 'center', justifyContent: 'space-between', gap: 12 },
        toolbarHistoryCopy: { flex: 1, gap: 12 },
        titleRow: { flexDirection: 'row', alignItems: 'center', gap: 6 },
        sectionTitle: { fontSize: 15, fontWeight: '600', color: colors.textPrimary },
        cardTitle: {
          marginHorizontal: 20,
          marginBottom: 8,
          fontSize: 13,
          fontWeight: '600',
          color: colors.textMuted,
          textTransform: 'uppercase',
        },
        tooltip: {
          position: 'absolute',
          left: 16,
          right: 16,
          padding: 12,
          borderRadius: 10,
          borderWidth: 1,
          borderColor: colors.border,
          backgroundColor: colors.surface,
          elevation: 8,
          shadowColor: '#000',
          shadowOpacity: 0.2,
          shadowRadius: 8,
          shadowOffset: { width: 0, height: 4 },
        },
        // A square turned 45 degrees, half of it hidden behind the bubble, so the bubble points at its icon.
        arrow: {
          position: 'absolute',
          top: -7,
          width: 12,
          height: 12,
          transform: [{ rotate: '45deg' }],
          borderLeftWidth: 1,
          borderTopWidth: 1,
          borderColor: colors.border,
          backgroundColor: colors.surface,
        },
        note: { flex: 1, fontSize: 13, fontWeight: '600', color: colors.warning },
        tooltipText: { fontSize: 14, color: colors.textPrimary },
        segmented: {
          flexDirection: 'row',
          backgroundColor: colors.background,
          borderRadius: 10,
          padding: 3,
        },
        option: { flex: 1, paddingVertical: 8, borderRadius: 8, alignItems: 'center' },
        optionActive: { backgroundColor: colors.accent },
        optionText: { fontSize: 14, fontWeight: '600', color: colors.textSecondary },
        optionTextActive: { color: '#fff' },
        clearButton: {
          alignSelf: 'center',
          borderWidth: 1,
          borderColor: colors.danger,
          borderRadius: 8,
          paddingHorizontal: 14,
          paddingVertical: 9,
        },
        clearButtonText: { color: colors.danger, fontSize: 14, fontWeight: '600' },
      }),
    [colors],
  );

  const settings: {
    /** The card the setting is in: settings with the same card share one. */
    card: string;
    key: string;
    title: string;
    help: string;
    /** A notice shown beside the help icon, or nothing when there is none. */
    note?: string;
    options: { value: string; label: string }[];
    value: string;
    onSelect: (value: any) => void;
  }[] = [
    {
      card: 'appearance',
      key: 'appearance',
      title: 'Appearance',
      help: 'Whether the app is light or dark. System follows your phone.',
      options: [
        { value: 'system', label: 'System' },
        { value: 'light', label: 'Light' },
        { value: 'dark', label: 'Dark' },
      ],
      value: preference,
      onSelect: setPreference,
    },
    {
      card: 'export',
      key: 'format',
      title: 'Export format',
      help: 'The file type of a photo when you share it. WebP makes smaller files with higher quality, JPEG works in every app, and PNG keeps every detail but is the biggest.',
      options: EXPORT_FORMATS,
      value: format,
      onSelect: value => exportFormat.next(value),
    },
    {
      card: 'export',
      key: 'quality',
      title: 'Export quality',
      help: 'How much detail a shared photo keeps. Best looks sharpest but makes the biggest file. It has no effect on PNG.',
      options: EXPORT_QUALITIES.map(option => ({ value: String(option.value), label: option.label })),
      value: String(quality),
      onSelect: value => exportQuality.next(Number(value)),
    },
    {
      card: 'performance',
      key: 'stack',
      title: 'Photo processing memory',
      help: 'How much working room each photo job gets. If the app closes by itself while it prepares a photo, try a bigger size. It only takes effect the next time you open the app.',
      options: WORKER_STACK_CHOICES.map(megabytes => ({ value: String(megabytes), label: `${megabytes} MB` })),
      note: stackInEffect !== null && stack !== stackInEffect ? 'Reopen the app to apply' : undefined,
      value: String(stack),
      onSelect: value => workerStackMegabytes.next(Number(value)),
    },
  ];

  return (
    <View style={styles.container}>
      <View style={commonStyles.header}>
        <AppText size="heading" bold>
          Settings
        </AppText>
      </View>

      <ScrollView contentContainerStyle={styles.cards}>
        {[...new Set(settings.map(setting => setting.card))].map(card => (
          <Fragment key={card}>
            <Text style={styles.cardTitle}>{CARD_TITLES[card] ?? card}</Text>
            <View style={styles.section}>
              {settings
                .filter(setting => setting.card === card)
                .map(setting => (
                  <View key={setting.key} style={styles.setting}>
                    <View style={styles.titleRow}>
                      <Text style={styles.sectionTitle}>{setting.title}</Text>
                      <Pressable
                        ref={icon => {
                          iconRefs.current[setting.key] = icon;
                        }}
                        accessibilityLabel={`About ${setting.title}`}
                        hitSlop={8}
                        onPress={() =>
                          iconRefs.current[setting.key]?.measureInWindow((x, y, width, height) =>
                            setTip({ key: setting.key, x: x + width / 2, y: y + height }),
                          )
                        }
                      >
                        <CircleQuestionMark color={colors.textMuted} size={16} />
                      </Pressable>
                      {setting.note !== undefined && <Text style={styles.note}>{setting.note}</Text>}
                    </View>
                    <View style={styles.segmented}>
                      {setting.options.map(option => {
                        const active = option.value === setting.value;
                        return (
                          <Pressable
                            key={option.value}
                            style={[styles.option, active && styles.optionActive]}
                            onPress={() => setting.onSelect(option.value)}
                          >
                            <Text style={[styles.optionText, active && styles.optionTextActive]}>{option.label}</Text>
                          </Pressable>
                        );
                      })}
                    </View>
                  </View>
                ))}
            </View>
          </Fragment>
        ))}
        <Text style={styles.cardTitle}>{CARD_TITLES.toolbar}</Text>
        <View style={styles.section}>
          <View style={styles.setting}>
            <View style={styles.toolbarHistoryRow}>
              <View style={styles.toolbarHistoryCopy}>
                <Text style={styles.sectionTitle}>Smart toolbar history</Text>
                <Text style={styles.tooltipText}>The toolbar is ordered by what you use most.</Text>
              </View>
              <Pressable
                accessibilityRole="button"
                accessibilityLabel="Clear smart toolbar history"
                style={styles.clearButton}
                onPress={() =>
                  Alert.alert('Clear toolbar history?', 'The toolbar will return to its default order.', [
                    { text: 'Cancel', style: 'cancel' },
                    { text: 'Clear history', style: 'destructive', onPress: clearToolbarUsage },
                  ])
                }
              >
                <Text style={styles.clearButtonText}>Clear history</Text>
              </Pressable>
            </View>
          </View>
        </View>
        <Text style={styles.cardTitle}>{CARD_TITLES.developer}</Text>
        <View style={styles.section}>
          <View style={styles.toolbarHistoryRow}>
            <View style={styles.toolbarHistoryCopy}>
              <Text style={styles.sectionTitle}>Developer mode</Text>
              <Text style={styles.tooltipText}>Show developer tools, including plugin live sync and the editor developer button.</Text>
            </View>
            <Switch
              accessibilityLabel="Developer mode"
              value={developerMode}
              onValueChange={toggleDeveloperMode}
              trackColor={{ true: colors.accent }}
            />
          </View>
        </View>
      </ScrollView>

      <Modal
        transparent
        statusBarTranslucent
        animationType="fade"
        visible={tip != null}
        onRequestClose={() => setTip(null)}
      >
        <Pressable style={StyleSheet.absoluteFill} onPress={() => setTip(null)}>
          {tip != null && (
            <View style={[styles.tooltip, { top: tip.y + 10 }]}>
              <View style={[styles.arrow, { left: tip.x - 16 - 6 }]} />
              <Text style={styles.tooltipText}>{settings.find(setting => setting.key === tip.key)?.help}</Text>
            </View>
          )}
        </Pressable>
      </Modal>
    </View>
  );
}
