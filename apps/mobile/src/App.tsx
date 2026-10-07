import { useState } from 'react';
import { StatusBar, StyleSheet, View } from 'react-native';
import { SafeAreaProvider, SafeAreaView } from 'react-native-safe-area-context';
import { BottomTabBar, type TabItem } from '@/src/components/bottom-tab-bar';
import { ThemeProvider, useTheme } from '@/src/lib/theme';
import EditScreen, { type EditablePhoto } from '@/src/screens/editor';
import HomeScreen from '@/src/screens/home';
import SettingsScreen from '@/src/screens/settings';
import StorageScreen from '@/src/screens/storage';

const TABS: TabItem[] = [
  { name: 'home', label: 'Home' },
  { name: 'settings', label: 'Settings' },
  { name: 'storage', label: 'Storage' },
];

export default function App() {
  return (
    <ThemeProvider>
      <SafeAreaProvider>
        <AppContent />
      </SafeAreaProvider>
    </ThemeProvider>
  );
}

function AppContent() {
  const { colors } = useTheme();
  const [activeTab, setActiveTab] = useState(TABS[0].name);
  // Editing takes over the whole screen (no tab bar), like a photo app's editor, so it's
  // tracked separately from the tabs rather than as one of them.
  const [editingPhoto, setEditingPhoto] = useState<EditablePhoto | null>(null);

  return (
    <>
      <StatusBar barStyle={colors.statusBarStyle} />
      {editingPhoto ? (
        <EditScreen photo={editingPhoto} onClose={() => setEditingPhoto(null)} />
      ) : (
        <SafeAreaView style={[styles.container, { backgroundColor: colors.background }]} edges={['top', 'bottom']}>
          <View style={styles.screen}>
            {activeTab === 'home' && <HomeScreen onOpenPhoto={setEditingPhoto} />}
            {activeTab === 'settings' && <SettingsScreen />}
            {activeTab === 'storage' && <StorageScreen />}
          </View>
          <BottomTabBar tabs={TABS} activeTab={activeTab} onTabPress={setActiveTab} />
        </SafeAreaView>
      )}
    </>
  );
}

const styles = StyleSheet.create({
  container: { flex: 1 },
  screen: { flex: 1 },
});
