import { StyleSheet, Text, TouchableOpacity, View } from 'react-native';

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
  return (
    <View style={styles.container}>
      {tabs.map(tab => (
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

const styles = StyleSheet.create({
  container: {
    flexDirection: 'row',
    backgroundColor: '#f5f5f5',
    borderTopWidth: 1,
    borderTopColor: '#e0e0e0',
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
    borderBottomColor: '#007AFF',
  },
  label: {
    fontSize: 12,
    color: '#666',
    fontWeight: '500',
  },
  activeLabel: {
    color: '#007AFF',
    fontWeight: '600',
  },
});
