import { useEffect, useRef, useState, type ReactNode } from 'react';
import { Modal, Pressable, StyleSheet, Switch, Text, TextInput, View } from 'react-native';
import { useObservable } from 'react-rx';
import { FontBrowser } from '@/src/components/font-browser';
import { STRIPE_AT, decodeWords, stripeValues, withWords, type StripeWords } from '@/src/effects/stripe-overlay';
import { useSliderPreview } from '@/src/hooks/useSliderPreview';
import { installedFamilies } from '@/src/lib/fonts/text-image';
import { leafControls, sliderValues, type SliderControl } from '@/src/lib/project/edit-sections';
import { STRIPE_SETTINGS_KEY } from '@/src/lib/project/edit-sections/keys';
import { stripeControl } from '@/src/lib/project/edit-sections/overlays';
import { useTheme } from '@/src/lib/ui/theme';
import { sliderCommitRequested, stripeTextRequested$ } from '@/src/state/commands';
import { adjustments, adjustments$ } from '@/src/state/edits';

/** The slider that holds the stripes' settings, which the words are saved into. */
const settings = leafControls([stripeControl]).find(
  (control): control is SliderControl => control.kind === 'slider' && control.key === STRIPE_SETTINGS_KEY,
)!;

/**
 * The dialogs where the main text or the sub text of the open stripes is set, shown when its card is tapped. Each change is shown on the photo as
 * it is made (the same live preview a slider uses); OK saves it as one step to undo, and Cancel puts the photo back as it was. The font is chosen
 * from the Fonts screen, as the collage plugin's inspector does, since the letters are drawn in a font downloaded there, and it is the same font
 * for both texts. The sub text can be turned off, and each text can be shown in capitals.
 */
export function StripeWordsDialog() {
  const { colors } = useTheme();
  const edits = useObservable(adjustments$, adjustments.value);
  const onLiveChange = useSliderPreview(settings);
  const saved = stripeValues(sliderValues(settings, edits[STRIPE_SETTINGS_KEY]));
  // Which text is open, or nothing when the dialog is closed.
  const [which, setWhich] = useState<'main' | 'sub' | null>(null);
  const [choosingFont, setChoosingFont] = useState(false);
  // Everything the dialog can change, as the full saved values, so Cancel is putting `saved` back.
  const [draft, setDraft] = useState<number[]>(saved);
  // The subscription lives as long as the component does, so it reads the latest values through a ref.
  const latest = useRef({ saved, onLiveChange });
  latest.current = { saved, onLiveChange };

  useEffect(() => {
    const subscription = stripeTextRequested$.subscribe(request => {
      const current = latest.current.saved;
      // A font that was deleted since it was chosen, or none chosen yet, shows as the first font on the device.
      const words = decodeWords(current);
      const family = installedFamilies().some(each => each.family === words.family) ? words.family : (installedFamilies()[0]?.family ?? '');
      setDraft(withWords(current, { ...words, family }));
      setWhich(request);
    });
    return () => subscription.unsubscribe();
  }, []);

  const show = (next: number[]) => {
    setDraft(next);
    latest.current.onLiveChange?.(next);
  };
  const words = decodeWords(draft);
  const setWords = (next: StripeWords) => show(withWords(draft, next));
  const setSwitch = (at: number, on: boolean) => show(draft.map((value, index) => (index === at ? (on ? 1 : 0) : value)));
  const finish = (save: boolean) => {
    setWhich(null);
    if (save) sliderCommitRequested.next({ control: settings, value: draft });
    else latest.current.onLiveChange?.(latest.current.saved);
  };

  const main = which === 'main';
  const key = main ? 'title' : 'subtitle';
  const capsAt = main ? STRIPE_AT.capsTitle : STRIPE_AT.capsSub;
  const row = (label: string, control: ReactNode) => (
    <View style={styles.row}>
      <Text style={{ color: colors.textSecondary }}>{label}</Text>
      {control}
    </View>
  );
  const toggle = (label: string, at: number) =>
    row(label, <Switch accessibilityLabel={label} value={draft[at] === 1} onValueChange={on => setSwitch(at, on)} style={styles.switch} />);

  return (
    <Modal visible={which !== null} transparent animationType="fade" onRequestClose={() => finish(false)}>
      <View style={styles.backdrop}>
        <View style={[styles.card, { backgroundColor: colors.surface }]}>
          <Text style={[styles.title, { color: colors.textPrimary }]}>{main ? 'Main Text' : 'Sub Text'}</Text>
          {!main && toggle('Show sub text', STRIPE_AT.showSub)}
          {(main || draft[STRIPE_AT.showSub] === 1) && (
            <>
              <TextInput
                accessibilityLabel={main ? 'Main text' : 'Sub text'}
                value={words[key]}
                onChangeText={typed => setWords({ ...words, [key]: typed })}
                autoCorrect={false}
                style={[styles.input, { color: colors.textPrimary, borderColor: colors.border }]}
              />
              {toggle('ALL CAPS', capsAt)}
            </>
          )}
          {row(
            'Font',
            <Pressable
              accessibilityRole="button"
              accessibilityLabel={`Font: ${words.family || 'none chosen'}`}
              onPress={() => setChoosingFont(true)}
              style={[styles.input, { backgroundColor: colors.chip, borderColor: colors.border }]}
            >
              <Text style={{ color: words.family ? colors.textPrimary : colors.textSecondary }}>{words.family || 'Download a font to write with'}</Text>
            </Pressable>,
          )}
          <View style={styles.buttons}>
            <Pressable accessibilityRole="button" onPress={() => finish(false)} style={styles.button}>
              <Text style={{ color: colors.textSecondary }}>Cancel</Text>
            </Pressable>
            <Pressable accessibilityRole="button" onPress={() => finish(true)} style={styles.button}>
              <Text style={{ color: colors.textPrimary, fontWeight: '600' }}>OK</Text>
            </Pressable>
          </View>
        </View>
      </View>
      <Modal visible={choosingFont} animationType="slide" onRequestClose={() => setChoosingFont(false)}>
        <FontBrowser
          selected={words.family}
          onPick={family => {
            setChoosingFont(false);
            setWords({ ...words, family });
          }}
          onClose={() => setChoosingFont(false)}
        />
      </Modal>
    </Modal>
  );
}

const styles = StyleSheet.create({
  backdrop: { flex: 1, justifyContent: 'center', alignItems: 'center', backgroundColor: 'rgba(0,0,0,0.5)', padding: 16 },
  card: { width: '100%', maxWidth: 420, borderRadius: 14, padding: 16, gap: 12 },
  title: { fontSize: 17, fontWeight: '600' },
  row: { gap: 4 },
  switch: { alignSelf: 'flex-start' },
  input: { borderWidth: 1, borderRadius: 8, paddingHorizontal: 10, paddingVertical: 8, fontSize: 15 },
  buttons: { flexDirection: 'row', justifyContent: 'flex-end', gap: 8 },
  button: { paddingHorizontal: 14, paddingVertical: 8 },
});
