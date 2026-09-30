import { ChevronLeft, Redo2, RedoDot, Undo2, UndoDot } from 'lucide-react-native';
import { useEffect, useMemo } from 'react';
import { Image, Pressable, ScrollView, StyleSheet, Text, View } from 'react-native';
import { useObservable } from 'react-rx';
import { EDIT_SECTIONS, findSection } from '@/src/lib/edit-sections';
import { commitSlider, redoHistory, toggleAction, undoHistory } from '@/src/lib/editor-history';
import { THUMBNAIL_SIZE } from '@/src/lib/editor-thumbnails';
import { useTheme } from '@/src/lib/theme';
import {
  blemishApplyRequested,
  redoRequested,
  redoRequested$,
  selectControlRequested,
  selectControlRequested$,
  selectSectionRequested,
  selectSectionRequested$,
  sliderCommitRequested,
  sliderCommitRequested$,
  undoRequested,
  undoRequested$,
} from '@/src/state/commands';
import {
  activeSectionKey,
  activeSectionKey$,
  adjustments$,
  appliedActions$,
  focusedControl$,
  focusedControlKey,
  isBlemishToolFocused$,
  resetVersion$,
} from '@/src/state/edits';
import { canRedo$, canRedoDraft$, canUndo$, canUndoDraft$ } from '@/src/state/history';
import { bottomInset$, controlThumbnails$ } from '@/src/state/preview';
import { useLiveSlider } from '@/src/hooks/useLiveSlider';
import { Slider } from '@/src/components/slider';
import { BlemishControls, BlemishToolController } from '@/src/components/tools/blemish-tool';

/** The bottom panel: history buttons, the focused control's editor, and the section tabs. */
export function EditorPanel() {
  const bottomInset = useObservable(bottomInset$, 0);
  const activeKey = useObservable(activeSectionKey$, 'section-colorization');
  const canUndoDraft = useObservable(canUndoDraft$, false);
  const canRedoDraft = useObservable(canRedoDraft$, false);
  const canUndo = useObservable(canUndo$, false);
  const canRedo = useObservable(canRedo$, false);
  const { colors } = useTheme();
  const styles = useMemo(() => createStyles(colors), [colors]);
  const activeSection = findSection(activeKey);
  const focusedControl = useObservable(focusedControl$, undefined);
  const isBlemishToolFocused = useObservable(isBlemishToolFocused$, false);
  const canUndoHistory = canUndoDraft || canUndo;
  const canRedoHistory = canRedoDraft || canRedo;

  useEffect(() => {
    const subscriptions = [
      undoRequested$.subscribe(undoHistory),
      redoRequested$.subscribe(redoHistory),
      selectSectionRequested$.subscribe(key => activeSectionKey.next(key)),
      selectControlRequested$.subscribe(control => {
        if (control.kind === 'action') toggleAction(control);
        else focusedControlKey.next(control.key);
      }),
      sliderCommitRequested$.subscribe(({ control, value }) => commitSlider(control, value)),
    ];
    return () => subscriptions.forEach(subscription => subscription.unsubscribe());
  }, []);

  return (
    <View style={[styles.panel, { paddingBottom: bottomInset + 12 }]}>
      <BlemishToolController />
      <View style={styles.history}>
        <Pressable
          style={styles.historyButton}
          onPress={() => undoRequested.next()}
          disabled={!canUndoHistory}
          hitSlop={8}
        >
          {canUndoDraft ? (
            <UndoDot color={colors.textPrimary} size={20} />
          ) : (
            <Undo2 color={canUndo ? colors.textPrimary : colors.textTertiary} size={20} />
          )}
        </Pressable>
        <Pressable
          style={styles.historyButton}
          onPress={() => redoRequested.next()}
          disabled={!canRedoHistory}
          hitSlop={8}
        >
          {canRedoDraft ? (
            <RedoDot color={colors.textPrimary} size={20} />
          ) : (
            <Redo2 color={canRedo ? colors.textPrimary : colors.textTertiary} size={20} />
          )}
        </Pressable>
      </View>
      <View style={styles.controls}>
        {isBlemishToolFocused ? (
          <BlemishControls onBack={() => focusedControlKey.next(null)} onApply={() => blemishApplyRequested.next()} />
        ) : focusedControl?.kind === 'slider' ? (
          <SliderEditor />
        ) : activeSection.previewThumbnails ? (
          <ThumbnailControls />
        ) : (
          <ChipControls />
        )}
      </View>
      <ScrollView horizontal showsHorizontalScrollIndicator={false} contentContainerStyle={styles.sections}>
        {EDIT_SECTIONS.map(section => (
          <Pressable
            key={section.key}
            style={[styles.sectionTab, section.key === activeKey && styles.sectionTabActive]}
            onPress={() => selectSectionRequested.next(section.key)}
          >
            <Text style={[styles.sectionText, section.key === activeKey && styles.sectionTextActive]}>
              {section.label}
            </Text>
          </Pressable>
        ))}
      </ScrollView>
    </View>
  );
}

function SliderEditor() {
  const control = useObservable(focusedControl$, undefined);
  const adjustments = useObservable(adjustments$, {});
  const resetVersion = useObservable(resetVersion$, 0);
  const { colors } = useTheme();
  const styles = useMemo(() => createStyles(colors), [colors]);
  const onLiveChange = useLiveSlider(control?.kind === 'slider' ? control : undefined);

  if (control?.kind !== 'slider') {
    return null;
  }

  return (
    <View style={styles.sliderRow}>
      <Pressable onPress={() => focusedControlKey.next(null)} hitSlop={8} style={styles.backChip}>
        <ChevronLeft color={colors.textPrimary} size={18} />
      </Pressable>
      <View style={styles.sliderFill}>
        <Slider
          key={control.key + '-' + resetVersion}
          label={control.label}
          min={control.min}
          max={control.max}
          step={control.step}
          initialValue={adjustments[control.key] ?? control.defaultValue}
          triggerType="release"
          onChange={onLiveChange}
          reset
          onTrigger={value => sliderCommitRequested.next({ control, value })}
          onReset={() => sliderCommitRequested.next({ control, value: control.defaultValue })}
        />
      </View>
    </View>
  );
}

function ThumbnailControls() {
  const sectionKey = useObservable(activeSectionKey$, 'section-colorization');
  const appliedActions = useObservable(appliedActions$, []);
  const controlThumbnails = useObservable(controlThumbnails$, {});
  const { colors } = useTheme();
  const styles = useMemo(() => createStyles(colors), [colors]);
  const activeSection = findSection(sectionKey);

  return (
    <ScrollView
      key={sectionKey}
      horizontal
      showsHorizontalScrollIndicator={false}
      contentContainerStyle={styles.thumbnailRow}
    >
      {activeSection.controls.map(control => {
        const adjusted = control.kind === 'action' && appliedActions.includes(control.key);
        const thumbnail = controlThumbnails[control.key];
        return (
          <Pressable
            key={control.key}
            style={styles.thumbnailButton}
            onPress={() => selectControlRequested.next(control)}
          >
            <View style={[styles.thumbnailImage, adjusted && styles.thumbnailApplied]}>
              {thumbnail && (
                <Image
                  source={{ uri: thumbnail }}
                  style={styles.thumbnailCanvas}
                  resizeMode="cover"
                  // Every edit swaps in freshly rendered thumbnails; Android's default 300ms fade-in would
                  // make the whole row fade out and back in on each tap.
                  fadeDuration={0}
                />
              )}
            </View>
            <Text style={[styles.thumbnailLabel, adjusted && styles.thumbnailLabelApplied]} numberOfLines={2}>
              {control.label}
            </Text>
          </Pressable>
        );
      })}
    </ScrollView>
  );
}

function ChipControls() {
  const sectionKey = useObservable(activeSectionKey$, 'section-colorization');
  const appliedActions = useObservable(appliedActions$, []);
  const adjustments = useObservable(adjustments$, {});
  const { colors } = useTheme();
  const styles = useMemo(() => createStyles(colors), [colors]);
  const activeSection = findSection(sectionKey);

  return (
    <ScrollView
      key={sectionKey}
      horizontal
      showsHorizontalScrollIndicator={false}
      contentContainerStyle={styles.chipRow}
    >
      {activeSection.controls.map(control => {
        const adjusted =
          control.kind === 'action'
            ? appliedActions.includes(control.key)
            : control.kind === 'slider' && (adjustments[control.key] ?? control.defaultValue) !== control.defaultValue;
        return (
          <Pressable
            key={control.key}
            style={[styles.chip, adjusted && styles.chipApplied]}
            onPress={() => selectControlRequested.next(control)}
          >
            <Text style={[styles.chipText, adjusted && styles.chipTextApplied]}>{control.label}</Text>
          </Pressable>
        );
      })}
    </ScrollView>
  );
}

function createStyles(colors: ReturnType<typeof useTheme>['colors']) {
  return StyleSheet.create({
    panel: { backgroundColor: colors.surface, paddingTop: 10 },
    history: { height: 36, flexDirection: 'row', justifyContent: 'center', alignItems: 'center', gap: 24 },
    historyButton: { padding: 6 },
    controls: { height: 96, justifyContent: 'center' },
    chipRow: { paddingHorizontal: 16, gap: 8, alignItems: 'center' },
    chip: { paddingHorizontal: 14, paddingVertical: 8, borderRadius: 16, backgroundColor: colors.chip },
    chipApplied: { backgroundColor: colors.accent },
    chipText: { color: colors.textSecondary, fontSize: 13, fontWeight: '500' },
    chipTextApplied: { color: '#fff', fontWeight: '700' },
    thumbnailRow: { paddingHorizontal: 16, gap: 12, alignItems: 'center' },
    thumbnailButton: { alignItems: 'center', gap: 4, width: THUMBNAIL_SIZE + 8 },
    thumbnailImage: {
      width: THUMBNAIL_SIZE,
      height: THUMBNAIL_SIZE,
      borderRadius: THUMBNAIL_SIZE / 2,
      overflow: 'hidden',
      backgroundColor: colors.chip,
      borderWidth: 2,
      borderColor: 'transparent',
    },
    thumbnailApplied: { borderColor: colors.accent },
    thumbnailCanvas: { width: THUMBNAIL_SIZE, height: THUMBNAIL_SIZE },
    thumbnailLabel: {
      color: colors.textSecondary,
      fontSize: 12,
      lineHeight: 14,
      height: 28,
      fontWeight: '500',
      textAlign: 'center',
    },
    thumbnailLabelApplied: { color: colors.textPrimary, fontWeight: '700' },
    sliderRow: { flexDirection: 'row', alignItems: 'center', paddingHorizontal: 12, gap: 10 },
    sliderFill: { flex: 1 },
    backChip: {
      width: 28,
      height: 28,
      borderRadius: 14,
      backgroundColor: colors.chip,
      alignItems: 'center',
      justifyContent: 'center',
    },
    sections: { paddingHorizontal: 16, gap: 8, paddingVertical: 10 },
    sectionTab: { paddingHorizontal: 16, paddingVertical: 8, borderRadius: 18 },
    sectionTabActive: { backgroundColor: colors.textPrimary },
    sectionText: { color: colors.textSecondary, fontSize: 14, fontWeight: '600' },
    sectionTextActive: { color: colors.background },
  });
}
