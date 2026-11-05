import { BottomTabBar, type TabItem } from '@/src/components/bottom-tab-bar';
import { ErrorBoundary } from '@/src/components/error-boundary';
import { syncAiModels } from '@/src/lib/ai/ai-model-catalog';
import { loadPlugins } from '@/src/lib/plugins/plugin-controls';
import { ThemeProvider, useTheme } from '@/src/lib/ui/theme';
import { EditScreen, type EditablePhoto } from '@/src/screens/editor';
import { FontsScreen } from '@/src/screens/fonts';
import { HomeScreen } from '@/src/screens/home';
import { PluginEditorScreen, type PluginWorkspaceScreenInfo } from '@/src/screens/plugin-editor';
import { PluginsScreen } from '@/src/screens/plugins';
import { SettingsScreen } from '@/src/screens/settings';
import { pluginEditorDevUpdate } from '@/src/state/plugin-editor';
import { restoreProjects } from '@/src/state/projects';
import { installDevPluginFolder } from '@alakazam/mobile';
import HomeIcon from 'lucide-react-native/icons/house';
import PluginsIcon from 'lucide-react-native/icons/plug';
import SettingsIcon from 'lucide-react-native/icons/settings';
import TypeIcon from 'lucide-react-native/icons/type';
import { useEffect, useState } from 'react';
import { Alert, NativeModules, StatusBar, StyleSheet, View } from 'react-native';
import BootSplash from 'react-native-bootsplash';
import { initialWindowMetrics, SafeAreaProvider, SafeAreaView } from 'react-native-safe-area-context';
import { errorMessage } from '@/src/lib/utils/error-message';
import { ColorPickerHost } from '@/src/components/color-picker/color-picker-host';
import { CurvePickerHost } from '@/src/components/curve-picker/curve-picker-host';
import { GradientPickerHost } from '@/src/components/gradient-picker/gradient-picker-host';

const TABS: TabItem[] = [
  { name: 'home', label: 'Home', icon: HomeIcon },
  { name: 'settings', label: 'Settings', icon: SettingsIcon },
  { name: 'fonts', label: 'Fonts', icon: TypeIcon },
  { name: 'plugins', label: 'Plugins', icon: PluginsIcon },
];

type DevPluginSync = {
  takeDevPluginFolder(): Promise<string | null>;
  clearDevPluginFolder(folder: string): Promise<void>;
};

export default function App() {
  return (
    <ThemeProvider>
      <SafeAreaProvider initialMetrics={initialWindowMetrics}>
        <ErrorBoundary>
          <AppContent />
        </ErrorBoundary>
        <CurvePickerHost />
        <GradientPickerHost />
        <ColorPickerHost />
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
  const [pluginWorkspace, setPluginWorkspace] = useState<PluginWorkspaceScreenInfo | null>(null);
  // The home screen waits for the saved projects, so it never shows its empty state before they appear.
  const [restored, setRestored] = useState(false);

  useEffect(() => {
    // The plugins' controls are in the editor before any photo opens.
    loadPlugins();
    restoreProjects().finally(() => setRestored(true));
  }, []);

  useEffect(() => {
    const sync = NativeModules.MediaFile as DevPluginSync | undefined;
    if (!sync?.takeDevPluginFolder || !sync.clearDevPluginFolder) return;
    let stopped = false;
    let busy = false;
    const poll = async () => {
      if (stopped || busy) return;
      busy = true;
      let folder: string | null = null;
      try {
        folder = await sync.takeDevPluginFolder();
        if (folder) {
          await installDevPluginFolder(folder);
          loadPlugins(true);
          syncAiModels();
          const folderName = folder.split(/[\\/]/).pop() ?? '';
          const pluginId = folderName.replace(
            /-[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\.installing$/i,
            '',
          );
          if (pluginId !== folderName) pluginEditorDevUpdate.next(pluginId);
        }
      } catch (error) {
        console.warn('Could not install the received plugin files:', errorMessage(error));
        Alert.alert('Plugin update failed', errorMessage(error));
      } finally {
        if (folder) {
          await sync
            .clearDevPluginFolder(folder)
            .catch(error => console.warn('Could not clear the received plugin folder:', error));
        }
        busy = false;
      }
    };
    const timer = setInterval(() => void poll(), 700);
    void poll();
    return () => {
      stopped = true;
      clearInterval(timer);
    };
  }, []);

  // The native splash covers the screen until the first real screen is ready to show.
  useEffect(() => {
    if (restored) BootSplash.hide({ fade: true });
  }, [restored]);

  if (!restored) return null;

  return (
    <>
      <StatusBar barStyle={colors.statusBarStyle} />
      <SafeAreaView
        style={[styles.container, { backgroundColor: colors.background }]}
        edges={['top', 'bottom']}
      >
        {pluginWorkspace ? (
          <PluginEditorScreen workspace={pluginWorkspace} onClose={() => setPluginWorkspace(null)} />
        ) : editingPhoto ? (
          <EditScreen photo={editingPhoto} onClose={() => setEditingPhoto(null)} />
        ) : (
          <View style={styles.screen}>
            {activeTab === 'home' && (
              <HomeScreen onOpenPhoto={setEditingPhoto} onOpenPluginEditor={setPluginWorkspace} />
            )}
            {activeTab === 'settings' && <SettingsScreen />}
            {activeTab === 'fonts' && <FontsScreen />}
            {activeTab === 'plugins' && <PluginsScreen />}
          </View>
        )}
        {!pluginWorkspace && !editingPhoto && (
          <BottomTabBar tabs={TABS} activeTab={activeTab} onTabPress={setActiveTab} />
        )}
      </SafeAreaView>
    </>
  );
}

const styles = StyleSheet.create({
  container: { flex: 1 },
  screen: { flex: 1 },
});
