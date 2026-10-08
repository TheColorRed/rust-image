import { useMemo } from 'react';
import { StyleSheet } from 'react-native';
import { useTheme } from '@/src/lib/theme';

/** Layout and component styles shared by more than one screen: headers, buttons, empty states. */
export function useCommonStyles() {
  const { colors } = useTheme();
  return useMemo(
    () =>
      StyleSheet.create({
        header: {
          flexDirection: 'row',
          alignItems: 'baseline',
          justifyContent: 'space-between',
          paddingHorizontal: 16,
          paddingVertical: 12,
        },
        headerTitle: { fontSize: 28, fontWeight: '700', color: colors.textPrimary },
        headerSubtitle: { fontSize: 13, color: colors.textMuted },

        button: {
          backgroundColor: colors.accent,
          paddingHorizontal: 20,
          paddingVertical: 10,
          borderRadius: 20,
        },
        buttonText: { color: '#fff', fontWeight: '600' },

        centered: { flex: 1, alignItems: 'center', justifyContent: 'center', paddingHorizontal: 32, gap: 8 },
        emptyTitle: { fontSize: 17, fontWeight: '600', color: colors.textPrimary },
        emptyBody: { fontSize: 14, color: colors.textMuted, textAlign: 'center' },
      }),
    [colors],
  );
}
