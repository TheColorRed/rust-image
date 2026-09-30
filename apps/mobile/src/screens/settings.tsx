import { useMemo } from 'react';
import { Pressable, StyleSheet, Text, View } from 'react-native';
import AbraScreen from '@/src/screens/abra';
import { AppText } from '@/src/components/app-text';
import { useCommonStyles } from '@/src/lib/styles';
import { useTheme, type ThemePreference } from '@/src/lib/theme';

const OPTIONS: { value: ThemePreference; label: string }[] = [
  { value: 'system', label: 'System' },
  { value: 'light', label: 'Light' },
  { value: 'dark', label: 'Dark' },
];

export default function SettingsScreen() {
  const { colors, preference, setPreference } = useTheme();
  const commonStyles = useCommonStyles();
  const styles = useMemo(
    () =>
      StyleSheet.create({
        container: { flex: 1, backgroundColor: colors.background, paddingTop: 24 },
        abra: { flex: 1, marginTop: 24 },
        section: { paddingHorizontal: 16, gap: 12 },
        sectionTitle: { fontSize: 13, fontWeight: '600', color: colors.textMuted, textTransform: 'uppercase' },
        segmented: {
          flexDirection: 'row',
          backgroundColor: colors.surface,
          borderRadius: 10,
          padding: 3,
        },
        option: { flex: 1, paddingVertical: 8, borderRadius: 8, alignItems: 'center' },
        optionActive: { backgroundColor: colors.accent },
        optionText: { fontSize: 14, fontWeight: '600', color: colors.textSecondary },
        optionTextActive: { color: '#fff' },
      }),
    [colors],
  );

  return (
    <View style={styles.container}>
      <View style={commonStyles.header}>
        <AppText size="heading" bold>
          ⚙️ Settings
        </AppText>
      </View>

      <View style={styles.section}>
        <Text style={styles.sectionTitle}>Appearance</Text>
        <View style={styles.segmented}>
          {OPTIONS.map((option) => {
            const active = option.value === preference;
            return (
              <Pressable
                key={option.value}
                style={[styles.option, active && styles.optionActive]}
                onPress={() => setPreference(option.value)}
              >
                <Text style={[styles.optionText, active && styles.optionTextActive]}>{option.label}</Text>
              </Pressable>
            );
          })}
        </View>
      </View>

      <View style={[styles.section, styles.abra]}>
        <Text style={styles.sectionTitle}>Abra live preview</Text>
        <AbraScreen />
      </View>
    </View>
  );
}
