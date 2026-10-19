import { BottomTabBar, type TabItem } from '@/src/components/bottom-tab-bar';
import { ThemeProvider, useTheme } from '@/src/lib/theme';
import { EditScreen, type EditablePhoto } from '@/src/screens/editor';
import { HomeScreen } from '@/src/screens/home';
import { SettingsScreen } from '@/src/screens/settings';
import { StorageScreen } from '@/src/screens/storage';
import { restoreProjects } from '@/src/state/projects';
import BootSplash from 'react-native-bootsplash';
import { useEffect, useState } from 'react';
import { StatusBar, StyleSheet, View } from 'react-native';
import { SafeAreaProvider, SafeAreaView } from 'react-native-safe-area-context';

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
  // The home screen waits for the saved projects, so it never shows its empty state before they appear.
  const [restored, setRestored] = useState(false);

  useEffect(() => {
    restoreProjects().finally(() => setRestored(true));
  }, []);

  // The native splash covers the screen until the first real screen is ready to show.
  useEffect(() => {
    if (restored) BootSplash.hide({ fade: true });
  }, [restored]);

  if (!restored) return null;

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
