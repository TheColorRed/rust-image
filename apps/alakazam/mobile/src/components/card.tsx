import { useTheme } from '@/src/lib/theme';
import type { LucideIcon } from 'lucide-react-native';
import { Children, useMemo, type ReactNode } from 'react';
import { StyleSheet, Text, TouchableOpacity, View, type StyleProp, type ViewStyle } from 'react-native';

export interface CardProps {
  /** Shown in a rounded tile at the start of the header. */
  icon?: LucideIcon;
  title?: string;
  subtitle?: string;
  /** Makes the whole card a button. */
  onPress?: () => void;
  /** Anything at all, shown below the header (or on its own when there is no header). */
  children?: ReactNode;
  /** Applied to the outer container, so a card can size itself in a row, a grid or a list. */
  style?: StyleProp<ViewStyle>;
}

export function CardContent({ children }: { children?: ReactNode }) {
  const styles = useMemo(
    () =>
      StyleSheet.create({
        content: { padding: 16 },
      }),
    [],
  );
  return <View style={styles.content}>{children}</View>;
}

export function CardHeader({ icon: Icon, title, subtitle }: { icon?: LucideIcon; title?: string; subtitle?: string }) {
  const { colors } = useTheme();
  const styles = useMemo(
    () =>
      StyleSheet.create({
        header: { flexDirection: 'row', flexWrap: 'wrap', alignItems: 'center', columnGap: 16, rowGap: 8, padding: 16 },
        icon: {
          width: 56,
          height: 56,
          alignItems: 'center',
          justifyContent: 'center',
          borderRadius: 14,
          backgroundColor: colors.chip,
        },
        text: { flexGrow: 1, flexShrink: 1, flexBasis: 0, minWidth: 140 },
        title: { fontSize: 18, fontWeight: '600', color: colors.textPrimary },
        subtitle: { marginTop: 4, fontSize: 13, color: colors.textSecondary },
      }),
    [colors],
  );

  const hasHeader = Icon != null || title != null || subtitle != null;
  if (!hasHeader) return null;

  return (
    <View style={styles.header}>
      {Icon != null && (
        <View style={styles.icon}>
          <Icon size={32} color={colors.accent} strokeWidth={1.8} />
        </View>
      )}
      {(title != null || subtitle != null) && (
        <View style={styles.text}>
          {title != null && <Text style={styles.title}>{title}</Text>}
          {subtitle != null && <Text style={styles.subtitle}>{subtitle}</Text>}
        </View>
      )}
    </View>
  );
}

/** Tells Card to always draw this first, wherever it sits among the children. */
CardHeader.slot = 'header';

/** A surface with an optional header (icon, title, subtitle), any content below it, and an optional tap target. */
export function Card({ icon: Icon, title, subtitle, onPress, children, style }: CardProps) {
  const { colors } = useTheme();
  const styles = useMemo(
    () =>
      StyleSheet.create({
        card: {
          borderWidth: 1,
          borderColor: colors.border,
          borderRadius: 16,
          backgroundColor: colors.surface,
          overflow: 'hidden',
        },
        icon: {
          width: 56,
          height: 56,
          alignItems: 'center',
          justifyContent: 'center',
          borderRadius: 14,
          backgroundColor: colors.chip,
        },
        text: { flex: 1 },
        title: { fontSize: 18, fontWeight: '600', color: colors.textPrimary },
        subtitle: { marginTop: 4, fontSize: 13, color: colors.textSecondary },
      }),
    [colors],
  );

  // Matched by marker, not by function identity: Fast Refresh swaps the function and the identity check stops matching.
  const isHeader = (child: any) => child?.type?.slot === 'header';
  const Header = Children.toArray(children).find(isHeader);
  const Content = Children.toArray(children).filter(child => !isHeader(child));
  const content = (
    <>
      {Header != null && Header}
      {Content.length > 0 && <CardContent>{Content}</CardContent>}
    </>
  );

  return typeof onPress === 'function' ? (
    <TouchableOpacity
      accessibilityRole="button"
      accessibilityLabel={subtitle ? `${title}. ${subtitle}` : title}
      activeOpacity={0.75}
      onPress={onPress}
      style={[styles.card, style]}
    >
      {content}
    </TouchableOpacity>
  ) : (
    <View style={[styles.card, style]}>{content}</View>
  );
}
