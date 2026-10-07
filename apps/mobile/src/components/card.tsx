import { ImagePlus } from 'lucide-react-native';
import { useMemo, type ReactNode } from 'react';
import { StyleSheet, Text, TouchableOpacity, View } from 'react-native';
import { useTheme } from '@/src/lib/theme';

export interface CardProps {
  title: string;
  subtitle?: string;
  onPress?: () => void;
  /** Turns the card into a titled group that holds these rows instead of an icon, subtitle and tap target. */
  children?: ReactNode;
}

export function Card({ title, subtitle, onPress, children }: CardProps) {
  const { colors } = useTheme();
  const styles = useMemo(
    () =>
      StyleSheet.create({
        card: {
          minHeight: 96,
          flexDirection: 'row',
          alignItems: 'center',
          gap: 16,
          paddingHorizontal: 18,
          paddingVertical: 16,
          borderWidth: 1,
          borderColor: colors.border,
          borderRadius: 16,
          backgroundColor: colors.surface,
        },
        icon: {
          width: 56,
          height: 56,
          alignItems: 'center',
          justifyContent: 'center',
          borderRadius: 14,
          backgroundColor: colors.chip,
        },
        title: { fontSize: 18, fontWeight: '600', color: colors.textPrimary },
        subtitle: { marginTop: 4, fontSize: 13, color: colors.textSecondary },
        content: { flex: 1 },
        group: {
          borderWidth: 1,
          borderColor: colors.border,
          borderRadius: 16,
          backgroundColor: colors.surface,
          overflow: 'hidden',
        },
        groupTitle: { padding: 16, fontSize: 18, fontWeight: '600', color: colors.textPrimary },
      }),
    [colors],
  );

  if (children) {
    return (
      <View style={styles.group}>
        <Text style={styles.groupTitle}>{title}</Text>
        {children}
      </View>
    );
  }

  const content = (
    <>
      <View style={styles.icon}>
        <ImagePlus size={32} color={colors.accent} strokeWidth={1.8} />
      </View>
      <View style={styles.content}>
        <Text style={styles.title}>{title}</Text>
        {subtitle && <Text style={styles.subtitle}>{subtitle}</Text>}
      </View>
    </>
  );

  return typeof onPress === 'function' ? (
    <TouchableOpacity
      accessibilityRole="button"
      accessibilityLabel={subtitle ? `${title}. ${subtitle}` : title}
      activeOpacity={0.75}
      onPress={onPress}
      style={styles.card}
    >
      {content}
    </TouchableOpacity>
  ) : (
    <View style={styles.card}>{content}</View>
  );
}
