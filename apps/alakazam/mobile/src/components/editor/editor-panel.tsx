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
  isMultiSlider,
  isSliderApplied,
  sliderDefault,
  sliderSpecs,
  sliderValues,
} from '@/src/lib/edit-sections';
import { commitSlider, redoHistory, removeSlider, toggleAction, undoHistory } from '@/src/lib/editor-history';
import { editorLive } from '@/src/lib/editor-live';
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
  historyVersion as historyVersionState,
  appliedActions,
  appliedActions$,
  focusedControlKey,
  focusedControlKey$,
  historyVersion$,
  isSkinControlFocused$,
} from '@/src/state/edits';
import { canRedo$, canUndo$ } from '@/src/state/history';
import { personOutlinesVisible, personSelection, personSelection$ } from '@/src/state/person-selection';
import { bottomInset$, controlThumbnails$ } from '@/src/state/preview';
import { replayHidden } from '@/src/state/session';
import { VesselView } from '@vessel/react-native';
import { ChevronLeft, Redo2, Undo2, X } from 'lucide-react-native';
import { useEffect, useMemo, useRef, useState } from 'react';
import { Pressable, ScrollView, StyleSheet, Text, View } from 'react-native';
import { useObservable, useSyncObservable } from 'react-rx';

/** The control that is open in the active section, or undefined. */
function useFocusedControl() {
  const sectionKey = useObservable(activeSectionKey$, activeSectionKey.value);
  const key = useObservable(focusedControlKey$, focusedControlKey.value);
  return key ? findSection(sectionKey).controls.find(control => control.key === key) : undefined;
}

/** The bottom panel: history buttons, the focused control's editor, and the section tabs. */
export function EditorPanel() {
  const bottomInset = useObservable(bottomInset$, 0);
  const activeKey = useObservable(activeSectionKey$, 'section-colorization');
  const canUndo = useObservable(canUndo$, false);
  const canRedo = useObservable(canRedo$, false);
  const { colors } = useTheme();
  const styles = useMemo(() => createStyles(colors), [colors]);
  const activeSection = findSection(activeKey);
  useLiveAction();
  const focusedControl = useFocusedControl();
  const isSkinControlFocused = useObservable(isSkinControlFocused$, false);

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
          if (control.live || control.tool) {
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
            commitSlider(control, sliderDefault(control));
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
        <Pressable style={styles.historyButton} onPress={() => undoRequested.next()} disabled={!canUndo} hitSlop={8}>
          <Undo2 color={canUndo ? colors.textPrimary : colors.textTertiary} size={20} />
        </Pressable>
        <Pressable style={styles.historyButton} onPress={() => redoRequested.next()} disabled={!canRedo} hitSlop={8}>
          <Redo2 color={canRedo ? colors.textPrimary : colors.textTertiary} size={20} />
        </Pressable>
      </View>
      <View
        style={[
          styles.controls,
          focusedControl?.kind === 'slider'
            ? styles.sliderControls
            : activeSection.previewThumbnails && !focusedControl
              ? styles.thumbnailControls
              : styles.fixedControls,
        ]}
      >
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
  const control = useFocusedControl();
  // Each starts from what the subject holds now: the hook's own first value is the one given here, and a control that opens on
  // an empty one would start its sliders from the defaults and keep them.
  const adjustments = useObservable(adjustments$, adjustmentsState.value);
  const selection = useObservable(personSelection$, personSelection.value);
  const historyVersion = useObservable(historyVersion$, historyVersionState.value);
  const { colors } = useTheme();
  const styles = useMemo(() => createStyles(colors), [colors]);
  const onLiveChange = useLiveSlider(control?.kind === 'slider' ? control : undefined);
  const preview = useSyncObservable(editorLive.view, null);
  // The value of every slider as the user has set them, for a control with several: moving one sends all of them.
  const sliderState = useRef<{ resetKey: string; values: number[] }>({ resetKey: '', values: [] });
  // Which slider a control that shows them one at a time (`display: 'tabulated'`) has open.
  const [selectedSlider, setSelectedSlider] = useState({ key: '', index: 0 });

  if (control?.kind !== 'slider') {
    return null;
  }
  const targetKey = adjustmentKey(control.key, selection.selectedId);
  const specs = sliderSpecs(control);
  // A different control, person or undo state starts the sliders again from what is committed.
  const resetKey = targetKey + '-' + historyVersion;
  if (sliderState.current.resetKey !== resetKey) {
    sliderState.current = { resetKey, values: sliderValues(control, adjustments[targetKey]) };
  }
  const values = sliderState.current.values;
  const tabulated = control.display === 'tabulated' && specs.length > 1;
  // A different control starts on its first slider.
  const openSlider = selectedSlider.key === control.key ? Math.min(selectedSlider.index, specs.length - 1) : 0;
  const committed = sliderValues(control, adjustments[targetKey]);
  /** What is committed for the control: a number for one slider, an array for several. */
  const committedValue = () => (isMultiSlider(control) ? [...values] : values[0]);

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
              key={targetKey + '-' + historyVersion}
              preview={preview}
              value={sliderValues(control, adjustments[targetKey])[0]}
              onTouch={() => editorLive.show()}
              onRelease={value => sliderCommitRequested.next({ control, value })}
            />
          </View>
        </>
      ) : (
        <View style={styles.sliderStack}>
          {isMultiSlider(control) && (
            // The back and close buttons belong to the control, so with several sliders they get a header of their own and
            // every slider looks the same.
            <View style={styles.sliderHeader}>
              {back}
              <Text style={styles.sliderTitle}>{control.label}</Text>
              {close}
            </View>
          )}
          {tabulated && (
            <View style={styles.sliderPills}>
              {specs.map((spec, index) => (
                <Pressable
                  key={index}
                  style={[styles.sliderPill, index === openSlider && styles.sliderPillOpen]}
                  onPress={() => setSelectedSlider({ key: control.key, index })}
                  accessibilityRole="tab"
                  accessibilityState={{ selected: index === openSlider }}
                >
                  <Text style={[styles.sliderPillText, index === openSlider && styles.sliderPillTextOpen]}>
                    {spec.label}
                  </Text>
                  {/* A dot marks a slider that is off its default, as the committed value has it. */}
                  {committed[index] !== spec.defaultValue && (
                    <View style={[styles.sliderPillDot, index === openSlider && styles.sliderPillDotOpen]} />
                  )}
                </Pressable>
              ))}
            </View>
          )}
          {specs.map((spec, index) =>
            tabulated && index !== openSlider ? null : (
              <Slider
                key={resetKey + '-' + index}
                // A single slider carries the buttons in its own header.
                leading={!isMultiSlider(control) && index === 0 ? back : undefined}
                trailing={!isMultiSlider(control) && index === 0 ? close : undefined}
                label={spec.label}
                min={spec.min}
                max={spec.max}
                step={spec.step}
                initialValue={values[index]}
                resetValue={spec.defaultValue}
                triggerType="release"
                onChange={value => {
                  values[index] = value;
                  onLiveChange?.([...values]);
                }}
                reset
                onTrigger={value => {
                  values[index] = value;
                  sliderCommitRequested.next({ control, value: committedValue() });
                }}
                onReset={() => {
                  values[index] = spec.defaultValue;
                  sliderCommitRequested.next({ control, value: committedValue() });
                }}
              />
            ),
          )}
        </View>
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
    controls: { justifyContent: 'center' },
    fixedControls: { height: 96 },
    // An open slider control is as tall as its sliders, so several of them do not spill over the buttons above and the tabs below.
    sliderControls: { minHeight: 96 },
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
    sliderStack: { gap: 4 },
    sliderPills: { flexDirection: 'row', gap: 8, paddingVertical: 6 },
    sliderPill: {
      flexDirection: 'row',
      alignItems: 'center',
      gap: 6,
      paddingHorizontal: 12,
      paddingVertical: 6,
      borderRadius: 14,
      backgroundColor: colors.chip,
    },
    sliderPillOpen: { backgroundColor: colors.accent },
    sliderPillText: { color: colors.textSecondary, fontSize: 13, fontWeight: '600' },
    sliderPillTextOpen: { color: '#fff' },
    sliderPillDot: { width: 6, height: 6, borderRadius: 3, backgroundColor: colors.accent },
    sliderPillDotOpen: { backgroundColor: '#fff' },
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
