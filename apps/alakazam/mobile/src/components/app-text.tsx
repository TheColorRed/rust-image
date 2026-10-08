import { useMemo } from 'react';
import { StyleSheet, Text } from 'react-native';
import { useTheme } from '@/src/lib/theme';

type AppTextProps = {
  children: React.ReactNode;
  size?: 'small' | 'medium' | 'large' | 'heading';
  bold?: boolean;
  color?: 'primary' | 'secondary' | 'tertiary';
  center?: boolean;
  className?: string; // ignored; Fabric doesn't support className without NativeWind
};

export function AppText({ children, size = 'medium', bold = false, color = 'primary', center = false }: AppTextProps) {
  const { colors } = useTheme();
  const styles = useMemo(
    () =>
      StyleSheet.create({
        sizeSmall: { fontSize: 14, marginBottom: 8 },
        sizeMedium: { fontSize: 16, marginBottom: 12 },
        sizeLarge: { fontSize: 18, marginBottom: 16 },
        sizeHeading: { fontSize: 20, marginBottom: 20 },
        bold: { fontWeight: 'bold' },
        center: { textAlign: 'center' },
        colorPrimary: { color: colors.textPrimary },
        colorSecondary: { color: colors.textSecondary },
        colorTertiary: { color: colors.textTertiary },
      }),
    [colors],
  );

  const sizeStyle =
    size === 'small'
      ? styles.sizeSmall
      : size === 'medium'
        ? styles.sizeMedium
        : size === 'large'
          ? styles.sizeLarge
          : styles.sizeHeading;

  const colorStyle =
    color === 'primary' ? styles.colorPrimary : color === 'secondary' ? styles.colorSecondary : styles.colorTertiary;

  return <Text style={[sizeStyle, colorStyle, center && styles.center, bold && styles.bold]}>{children}</Text>;
}
