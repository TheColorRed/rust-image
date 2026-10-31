import { useTheme } from '@/src/lib/ui/theme';
import type { LucideIcon } from 'lucide-react-native';
import { createElement, useMemo } from 'react';
import { StyleSheet, Text, TouchableOpacity, useWindowDimensions, View } from 'react-native';

// Below this much width per tab, the label no longer fits next to the icon.
const MIN_TAB_WIDTH_FOR_LABEL = 100;

export interface TabItem {
  name: string;
  label?: string;
  icon?: LucideIcon;
}

interface BottomTabBarProps {
  tabs: TabItem[];
  activeTab: string;
  onTabPress: (tabName: string) => void;
}

export function BottomTabBar({ tabs, activeTab, onTabPress }: BottomTabBarProps) {
  const { colors } = useTheme();
  const { width } = useWindowDimensions();
  const iconOnly = width / tabs.length < MIN_TAB_WIDTH_FOR_LABEL;
  const styles = useMemo(
    () =>
      StyleSheet.create({
        container: {
          flexDirection: 'row',
          backgroundColor: colors.surface,
          borderTopWidth: 1,
          borderTopColor: colors.border,
          paddingBottom: 8,
        },
        tab: {
          flex: 1,
          paddingVertical: 12,
          alignItems: 'center',
          justifyContent: 'center',
        },
        activeTab: {
          borderBottomWidth: 3,
          borderBottomColor: colors.accent,
        },
        label: {
          fontSize: 12,
          color: colors.textSecondary,
          fontWeight: '500',
        },
        activeLabel: {
          color: colors.accent,
          fontWeight: '600',
        },
      }),
    [colors],
  );

  return (
    <View style={styles.container}>
      {tabs.map(tab => (
        <TouchableOpacity
          key={tab.name}
          style={[styles.tab, activeTab === tab.name && styles.activeTab]}
          onPress={() => onTabPress(tab.name)}
          accessibilityRole="tab"
          accessibilityLabel={tab.label ?? tab.name}
        >
          <View style={{ alignItems: 'center', display: 'flex', flexDirection: 'row', gap: 4 }}>
            {tab.icon &&
              createElement(tab.icon, {
                color: activeTab === tab.name ? colors.accent : colors.textSecondary,
                size: 20,
              })}
            {tab.label && !(iconOnly && tab.icon) && <Text style={[styles.label, activeTab === tab.name && styles.activeLabel]}>{tab.label}</Text>}
          </View>
        </TouchableOpacity>
      ))}
    </View>
  );
}
