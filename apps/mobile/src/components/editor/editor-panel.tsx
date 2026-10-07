import { SkinTanPicker } from '@/src/components/editor/skin-tan-picker';
import { Slider } from '@/src/components/slider';
import { BlemishControls, BlemishToolController } from '@/src/components/tools/blemish-tool';
import { PersonSelectionControls, PersonSelectionToolController } from '@/src/components/tools/person-selection-tool';
import { useLiveAction } from '@/src/hooks/useLiveAction';
import { useLiveSlider } from '@/src/hooks/useLiveSlider';
import {
  BLEMISH_TOOL_KEY,
  EDIT_SECTIONS,
  appliesOnSelect,
  findSection,
  isSliderApplied,
} from '@/src/lib/edit-sections';
import { editorLive } from '@/src/lib/editor-live';
import { commitSlider, redoHistory, removeSlider, toggleAction, undoHistory } from '@/src/lib/editor-history';
import { THUMBNAIL_HEIGHT, THUMBNAIL_WIDTH } from '@/src/lib/editor-thumbnails';
import { adjustmentKey, isSkinAdjustment, skinAdjustmentTargets } from '@/src/lib/skin-adjustments';
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
  isSkinControlFocused$,
  resetVersion$,
} from '@/src/state/edits';
import { canRedo$, canRedoDraft$, canUndo$, canUndoDraft$ } from '@/src/state/history';
import { bottomInset$, controlThumbnails$ } from '@/src/state/preview';
import { personOutlinesVisible, personSelection, personSelection$ } from '@/src/state/person-selection';
import { replayHidden } from '@/src/state/session';
import { ChevronLeft, Redo2, RedoDot, Undo2, UndoDot, X } from 'lucide-react-native';
import { VesselView } from '@vessel/react-native';
import { useEffect, useMemo } from 'react';
import { Pressable, ScrollView, StyleSheet, Text, View } from 'react-native';
import { useObservable, useSyncObservable } from 'react-rx';

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
  const isSkinControlFocused = useObservable(isSkinControlFocused$, false);
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
          if (
            control.kind === 'slider' &&
            appliesOnSelect(control) &&
            adjustmentsState.value[control.key] === undefined
          ) {
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
      <PersonSelectionToolController />
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
      <View style={[styles.controls, activeSection.previewThumbnails && !focusedControl && styles.thumbnailControls]}>
        {focusedControl?.kind === 'tool' && focusedControl.key === BLEMISH_TOOL_KEY ? (
          <BlemishControls onBack={() => focusedControlKey.next(null)} onApply={() => blemishApplyRequested.next()} />
        ) : focusedControl?.kind === 'slider' ? (
          <>
            {isSkinControlFocused && <PersonSelectionControls />}
            <SliderEditor />
          </>
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
  const selection = useObservable(personSelection$, personSelection.value);
  const { colors } = useTheme();
  const styles = useMemo(() => createStyles(colors), [colors]);
  const onLiveChange = useLiveSlider(control?.kind === 'slider' ? control : undefined);
  const preview = useSyncObservable(editorLive.view, null);

  if (control?.kind !== 'slider') {
    return null;
  }
  const targetKey = adjustmentKey(control.key, selection.selectedId);

  const back = (
    <Pressable onPress={() => focusedControlKey.next(null)} hitSlop={8} style={styles.backChip}>
      <ChevronLeft color={colors.textPrimary} size={18} />
    </Pressable>
  );
  // Turns the effect off and closes the slider. The slider's own reset only puts the value back to its default.
  const close = (
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
  );

  return (
    <View
      style={styles.sliderPanel}
      onTouchStart={() => {
        if (personSelection.value.focused) personOutlinesVisible.next(false);
      }}
    >
      {control.picker === 'skin-tan' ? (
        <>
          <View style={styles.sliderHeader}>
            {back}
            <Text style={styles.sliderTitle}>{control.label}</Text>
            {close}
          </View>
          <View style={styles.sliderTrack}>
            <SkinTanPicker
              key={targetKey + '-' + resetVersion}
              preview={preview}
              value={adjustments[targetKey] ?? control.defaultValue}
              onTouch={() => editorLive.show()}
              onRelease={value => sliderCommitRequested.next({ control, value })}
            />
          </View>
        </>
      ) : (
        <Slider
          key={targetKey + '-' + resetVersion}
          leading={back}
          trailing={close}
          label={control.label}
          min={control.min}
          max={control.max}
          step={control.step ?? 1}
          initialValue={adjustments[targetKey] ?? control.defaultValue}
          resetValue={control.defaultValue}
          triggerType="release"
          onChange={onLiveChange}
          reset
          onTrigger={value => sliderCommitRequested.next({ control, value })}
          onReset={() => sliderCommitRequested.next({ control, value: control.defaultValue })}
        />
      )}
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
            accessibilityRole="button"
            accessibilityLabel={control.label}
            accessibilityState={{ selected: adjusted }}
          >
            <View style={[styles.thumbnailImage, adjusted && styles.thumbnailApplied]}>
              {VesselView && thumbnail ? (
                <VesselView source={thumbnail} style={styles.thumbnailCanvas} pointerEvents="none" />
              ) : (
                <Text style={styles.thumbnailPlaceholder}>{control.label}</Text>
              )}
            </View>
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
            : control.kind === 'slider' &&
              (isSkinAdjustment(control.key)
                ? skinAdjustmentTargets(adjustments, control.key).some(target => isSliderApplied(control, target.value))
                : isSliderApplied(control, adjustments[control.key]));
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
    thumbnailControls: { height: THUMBNAIL_HEIGHT + 16 },
    chipRow: { paddingHorizontal: 16, gap: 8, alignItems: 'center' },
    chip: { paddingHorizontal: 14, paddingVertical: 8, borderRadius: 16, backgroundColor: colors.chip },
    chipApplied: { backgroundColor: colors.accent },
    chipText: { color: colors.textSecondary, fontSize: 13, fontWeight: '500' },
    chipTextApplied: { color: '#fff', fontWeight: '700' },
    thumbnailRow: { paddingHorizontal: 16, gap: 12, alignItems: 'center' },
    thumbnailButton: { alignItems: 'center', width: THUMBNAIL_WIDTH },
    thumbnailImage: {
      width: THUMBNAIL_WIDTH,
      height: THUMBNAIL_HEIGHT,
      borderRadius: 8,
      borderWidth: 2,
      borderColor: 'transparent',
    },
    thumbnailApplied: { borderColor: colors.accent },
    thumbnailCanvas: { flex: 1 },
    thumbnailPlaceholder: { color: colors.textSecondary, textAlign: 'center', margin: 'auto' },
    sliderPanel: { paddingHorizontal: 12 },
    sliderHeader: { flexDirection: 'row', alignItems: 'center', gap: 10 },
    sliderTitle: { flex: 1, color: colors.textPrimary, fontSize: 14, fontWeight: '600' },
    // Same side margin as the slider track, so the tan bar also stays clear of the screen edge.
    sliderTrack: { height: 40, marginHorizontal: 20, justifyContent: 'center' },
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
