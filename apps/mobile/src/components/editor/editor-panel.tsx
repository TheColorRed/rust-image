import { Slider } from '@/src/components/slider';
import { BlemishControls, BlemishToolController } from '@/src/components/tools/blemish-tool';
import { useLiveAction } from '@/src/hooks/useLiveAction';
import { useLiveSlider } from '@/src/hooks/useLiveSlider';
import { EDIT_SECTIONS, appliesOnSelect, findSection, isSliderApplied } from '@/src/lib/edit-sections';
import { commitSlider, redoHistory, removeSlider, toggleAction, undoHistory } from '@/src/lib/editor-history';
import { THUMBNAIL_SIZE } from '@/src/lib/editor-thumbnails';
import { useTheme } from '@/src/lib/theme';
import {
  actionPreviewRequested,
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
  adjustments as adjustmentsState,
  appliedActions,
  appliedActions$,
  focusedControl$,
  focusedControlKey,
  isBlemishToolFocused$,
  resetVersion$,
} from '@/src/state/edits';
import { canRedo$, canRedoDraft$, canUndo$, canUndoDraft$ } from '@/src/state/history';
import { bottomInset$, controlThumbnails$ } from '@/src/state/preview';
import { replayHidden } from '@/src/state/session';
import { ChevronLeft, Redo2, RedoDot, Undo2, UndoDot, X } from 'lucide-react-native';
import { useEffect, useMemo, useState } from 'react';
import { Image, Pressable, ScrollView, StyleSheet, Text, View, type ImageStyle, type StyleProp } from 'react-native';
import { useObservable } from 'react-rx';

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
  useLiveAction();
  const focusedControl = useObservable(focusedControl$, undefined);
  const isBlemishToolFocused = useObservable(isBlemishToolFocused$, false);
  const canUndoHistory = canUndoDraft || canUndo;
  const canRedoHistory = canRedoDraft || canRedo;

  useEffect(() => {
    const subscriptions = [
      undoRequested$.subscribe(undoHistory),
      redoRequested$.subscribe(redoHistory),
      // Picking a section closes any open slider, including when it is the section already showing, so its controls come back.
      selectSectionRequested$.subscribe(key => {
        focusedControlKey.next(null);
        activeSectionKey.next(key);
      }),
      selectControlRequested$.subscribe(control => {
        if (control.kind === 'action') {
          if (control.live) {
            // Applying previews at once; removing has nothing to preview, but the same short replay needs no spinner either.
            replayHidden.next(true);
            if (!appliedActions.value.includes(control.key)) actionPreviewRequested.next(control);
          }
          toggleAction(control);
        } else {
          focusedControlKey.next(control.key);
          if (control.kind === 'slider' && appliesOnSelect(control) && adjustmentsState.value[control.key] === undefined) {
            commitSlider(control, control.defaultValue);
          }
        }
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
      {/* Turns the effect off and closes the slider. The slider's own reset only puts the value back to its default. */}
      <Pressable
        onPress={() => {
          removeSlider(control);
          focusedControlKey.next(null);
        }}
        hitSlop={8}
        style={styles.backChip}
      >
        <X color={colors.textPrimary} size={16} />
      </Pressable>
      <View style={styles.sliderFill}>
        <Slider
          key={control.key + '-' + resetVersion}
          label={control.label}
          min={control.min}
          max={control.max}
          step={control.step ?? 1}
          initialValue={adjustments[control.key] ?? control.defaultValue}
          resetValue={control.defaultValue}
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
              {thumbnail && <Thumbnail uri={thumbnail} style={styles.thumbnailCanvas} />}
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

/**
 * A thumbnail that keeps showing the previous image until the new one has loaded. Every edit swaps in freshly rendered
 * data URIs; an `Image` whose uri changes is blank while the new one decodes, which shows as a flicker.
 */
function Thumbnail({ uri, style }: { uri: string; style: StyleProp<ImageStyle> }) {
  const [shown, setShown] = useState(uri);
  // Android's default 300ms fade-in would make the row fade out and back in on each edit.
  return (
    <>
      <Image source={{ uri: shown }} style={[style, StyleSheet.absoluteFill]} resizeMode="cover" fadeDuration={0} />
      {uri !== shown && (
        <Image
          source={{ uri }}
          style={[style, StyleSheet.absoluteFill]}
          resizeMode="cover"
          fadeDuration={0}
          onLoad={() => setShown(uri)}
        />
      )}
    </>
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
            : control.kind === 'slider' && isSliderApplied(control, adjustments[control.key]);
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
