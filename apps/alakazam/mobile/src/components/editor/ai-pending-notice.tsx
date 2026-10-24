import { useTheme } from '@/src/lib/ui/theme';
import { aiProgress, aiProgress$ } from '@/src/state/ai-progress';
import { useEffect, useMemo, useState } from 'react';
import { ActivityIndicator, StyleSheet, Text, View } from 'react-native';
import { useObservable } from 'react-rx';

/** How long the AI works before the banner shows: short jobs finish before anyone would read it. */
const SHOW_AFTER_MS = 2000;

/** What each AI task finds, in plain words, by the name it has in `aiProgress`. */
const FINDING: Record<string, string> = {
  skin: 'the skin',
  people: 'the people',
  faces: 'the faces',
  hair: 'the hair',
  subject: 'the people and their clothes',
};

/**
 * A banner over the photo that says the AI is still getting the photo ready, once it has been working for `SHOW_AFTER_MS` without
 * finishing, and stops the editor from being used until it is done. It names the step that is running now, and the bar under the
 * photo shows how far along the work is. It is for the work that starts when a photo opens, and it goes away by itself when the work is done.
 */
export function AiPendingNotice({
  text = 'Getting your photo ready',
  subText = 'Your edits will start working by themselves when that is done.',
}: {
  text?: string;
  subText?: string;
}) {
  const { colors } = useTheme();
  const tasks = useObservable(aiProgress$, aiProgress.value);
  const unfinished = Object.entries(tasks).filter(([, progress]) => progress !== 1);
  const working = unfinished.length > 0;
  const [late, setLate] = useState(false);
  useEffect(() => {
    setLate(false);
    if (!working) return;
    const timer = setTimeout(() => setLate(true), SHOW_AFTER_MS);
    return () => clearTimeout(timer);
  }, [working]);
  const styles = useMemo(
    () =>
      StyleSheet.create({
        wrap: {
          position: 'absolute',
          top: 0,
          left: 0,
          right: 0,
          bottom: 0,
          justifyContent: 'center',
          paddingHorizontal: 16,
          backgroundColor: 'rgba(0,0,0,0.45)',
        },
        box: {
          alignSelf: 'center',
          width: '100%',
          maxWidth: 420,
          flexDirection: 'row',
          alignItems: 'center',
          gap: 14,
          padding: 18,
          borderRadius: 16,
          borderWidth: 2,
          borderColor: colors.accent,
          backgroundColor: colors.surface,
          shadowColor: '#000',
          shadowOpacity: 0.35,
          shadowRadius: 12,
          elevation: 12,
        },
        text: { flexShrink: 1, flexGrow: 1, gap: 4 },
        title: { fontSize: 17, fontWeight: '700', color: colors.textPrimary },
        body: { fontSize: 14, color: colors.textSecondary },
        step: { fontSize: 13, fontWeight: '600', color: colors.textPrimary },
      }),
    [colors],
  );

  if (!working || !late) return null;
  // The step that is running now, which gives way to the next one when that starts.
  const current = unfinished.map(([name]) => FINDING[name]).find(Boolean);

  return (
    // It covers the whole editor and takes every touch, so nothing underneath can be used until the work is done.
    <View style={styles.wrap} onStartShouldSetResponder={() => true} accessibilityRole="alert" accessibilityViewIsModal>
      <View style={styles.box}>
        <ActivityIndicator size="large" color={colors.accent} />
        <View style={styles.text}>
          <Text style={styles.title}>{text}</Text>
          <Text style={styles.body}>{subText}</Text>
          {current ? <Text style={styles.step}>Finding {current}…</Text> : null}
        </View>
      </View>
    </View>
  );
}
