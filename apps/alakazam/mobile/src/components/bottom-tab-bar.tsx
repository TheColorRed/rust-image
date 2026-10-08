import { useMemo } from 'react';
import { StyleSheet, Text, TouchableOpacity, View } from 'react-native';
import { useTheme } from '@/src/lib/theme';

export interface TabItem {
  name: string;
  label: string;
}

interface BottomTabBarProps {
  tabs: TabItem[];
  activeTab: string;
  onTabPress: (tabName: string) => void;
}

export function BottomTabBar({ tabs, activeTab, onTabPress }: BottomTabBarProps) {
  const { colors } = useTheme();
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
      {tabs.map((tab) => (
        <TouchableOpacity
          key={tab.name}
          style={[styles.tab, activeTab === tab.name && styles.activeTab]}
          onPress={() => onTabPress(tab.name)}
        >
          <Text style={[styles.label, activeTab === tab.name && styles.activeLabel]}>{tab.label}</Text>
        </TouchableOpacity>
      ))}
    </View>
  );
}
