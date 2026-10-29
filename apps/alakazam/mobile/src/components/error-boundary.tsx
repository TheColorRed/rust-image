import { useTheme } from '@/src/lib/ui/theme';
import { Component, useEffect, type ErrorInfo, type ReactNode } from 'react';
import { Pressable, ScrollView, StyleSheet, Text, View } from 'react-native';
import BootSplash from 'react-native-bootsplash';

/** What replaces a screen that threw while rendering: the error, and a button that renders the screen again from scratch. */
function ErrorFallback({ error, componentStack, onRetry }: { error: Error; componentStack: string; onRetry: () => void }) {
  const { colors } = useTheme();
  // The splash covers the screen until the first real screen is ready, which a crash may never reach.
  useEffect(() => {
    BootSplash.hide({ fade: false });
  }, []);
  return (
    <View style={[styles.screen, { backgroundColor: colors.background }]}>
      <Text style={[styles.title, { color: colors.textPrimary }]}>Something went wrong</Text>
      <ScrollView style={styles.details}>
        <Text selectable style={[styles.message, { color: colors.danger }]}>{error.message}</Text>
        <Text selectable style={[styles.stack, { color: colors.textSecondary }]}>{componentStack.trim()}</Text>
      </ScrollView>
      <Pressable accessibilityRole="button" onPress={onRetry} style={[styles.button, { backgroundColor: colors.accent }]}>
        <Text style={styles.buttonText}>Try again</Text>
      </Pressable>
    </View>
  );
}

/**
 * Catches an error thrown while rendering anything below it and shows `ErrorFallback` instead of leaving a blank screen. Errors in
 * event handlers and timers are not caught by React, so those still reach the dev error screen. Place it inside `ThemeProvider`.
 */
export class ErrorBoundary extends Component<{ children: ReactNode }, { error: Error | null; componentStack: string }> {
  state = { error: null as Error | null, componentStack: '' };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('A screen crashed while rendering:', error, info.componentStack);
    this.setState({ componentStack: info.componentStack ?? '' });
  }

  render() {
    const { error, componentStack } = this.state;
    if (error === null) return this.props.children;
    return <ErrorFallback error={error} componentStack={componentStack} onRetry={() => this.setState({ error: null, componentStack: '' })} />;
  }
}

const styles = StyleSheet.create({
  screen: { flex: 1, padding: 24, gap: 16, justifyContent: 'center' },
  title: { fontSize: 22, fontWeight: '700' },
  details: { flexGrow: 0, maxHeight: '60%' },
  message: { fontSize: 15, fontWeight: '600', marginBottom: 8 },
  stack: { fontSize: 12 },
  button: { minHeight: 48, borderRadius: 24, alignItems: 'center', justifyContent: 'center', paddingHorizontal: 28, alignSelf: 'flex-start' },
  buttonText: { color: '#fff', fontSize: 16, fontWeight: '600' },
});
