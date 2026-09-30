import { createContext, useContext, useMemo, useState, type ReactNode } from 'react';
import { useColorScheme } from 'react-native';

export type ColorScheme = 'light' | 'dark';
/** The user's choice in Settings; 'system' follows the OS setting. */
export type ThemePreference = ColorScheme | 'system';

export interface ThemeColors {
  background: string;
  surface: string;
  border: string;
  accent: string;
  textPrimary: string;
  textSecondary: string;
  textTertiary: string;
  textMuted: string;
  danger: string;
  success: string;
  placeholder: string;
  /** Translucent pill/chip background (e.g. the edit screen's section tabs). */
  chip: string;
  /**
   * Translucent scrim for buttons that float directly over photo content (the edit screen's
   * top bar). Held constant across themes: it's about legibility over arbitrary image colors,
   * not the app's chrome.
   */
  overlayButton: string;
  statusBarStyle: 'light-content' | 'dark-content';
}

const lightColors: ThemeColors = {
  background: '#ffffff',
  surface: '#f5f5f5',
  border: '#e0e0e0',
  accent: '#007AFF',
  textPrimary: '#000000',
  textSecondary: '#666666',
  textTertiary: '#9ca3af',
  textMuted: '#888888',
  danger: '#d22',
  success: '#2a7',
  placeholder: '#eeeeee',
  chip: 'rgba(0,0,0,0.06)',
  overlayButton: 'rgba(0,0,0,0.45)',
  statusBarStyle: 'dark-content',
};

const darkColors: ThemeColors = {
  background: '#000000',
  surface: '#1c1c1e',
  border: '#2c2c2e',
  accent: '#0A84FF',
  textPrimary: '#ffffff',
  textSecondary: '#aeaeb2',
  textTertiary: '#8e8e93',
  textMuted: '#98989d',
  danger: '#ff453a',
  success: '#32d74b',
  placeholder: '#2c2c2e',
  chip: 'rgba(255,255,255,0.12)',
  overlayButton: 'rgba(0,0,0,0.45)',
  statusBarStyle: 'light-content',
};

interface ThemeContextValue {
  colorScheme: ColorScheme;
  preference: ThemePreference;
  setPreference: (preference: ThemePreference) => void;
  colors: ThemeColors;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

export function ThemeProvider({ children }: { children: ReactNode }) {
  const systemScheme = useColorScheme();
  // Not persisted across app restarts — add @react-native-async-storage/async-storage to do that.
  const [preference, setPreference] = useState<ThemePreference>('system');
  const colorScheme: ColorScheme = preference === 'system' ? (systemScheme === 'dark' ? 'dark' : 'light') : preference;
  const colors = colorScheme === 'dark' ? darkColors : lightColors;

  const value = useMemo(
    () => ({ colorScheme, preference, setPreference, colors }),
    [colorScheme, preference, colors],
  );

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export function useTheme(): ThemeContextValue {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error('useTheme must be used within a ThemeProvider');
  return ctx;
}
