import { filter, takeUntil } from 'rxjs/operators';
import { ColorBarPicker } from '@/src/components/editor/color-bar-picker';
import { Slider } from '@/src/components/slider';
import { BlemishControls, BlemishToolController } from '@/src/components/tools/blemish-tool';
import { CropControls, CropToolController } from '@/src/components/tools/crop-tool';
import { RedEyeControls, RedEyeToolController } from '@/src/components/tools/red-eye-tool';
import { PersonEveryoneButton, PersonSelectionToolController } from '@/src/components/tools/person-selection-tool';
import { GLOW_COLORS, INFRARED_COLORS, SHADE_COLORS, TAN_COLORS, hueColors } from '@/src/lib/color-bars';
import { missingModels } from '@/src/lib/ai-model-availability';
import { MASK_KINDS, analysesUsedBy } from '@/src/lib/ai-analyses';
import { HAIR_MODELS, MAKEUP_MODELS } from '@/src/lib/faces';
import { aiModelList, aiModelList$, aiModelsRequested } from '@/src/state/ai-models';
import { aiProgress, aiProgress$ } from '@/src/state/ai-progress';
import {
  CONTOUR_KEY,
  HIGHLIGHTER_KEY,
  BLUSH_KEY,
  LIP_LINER_KEY,
  BROW_BONE_KEY,
  EYELINER_KEY,
  EYESHADOW_KEY,
  HAIR_COLOR_KEY,
  LIPSTICK_KEY,
  UNDER_EYE_KEY,
} from '@/src/lib/edit-sections/keys';
import { useActionPreview } from '@/src/hooks/useActionPreview';
import { useSliderPreview } from '@/src/hooks/useSliderPreview';
import {
  BLEMISH_TOOL_KEY,
  CROP_TOOL_KEY,
  RED_EYE_TOOL_KEY,
  EDIT_SECTIONS,
  appliesOnSelect,
  groupPath,
  findSection,
  isMultiSlider,
  type EditControl,
  type GroupControl,
  isSliderApplied,
  leafControls,
  sliderDefault,
  sliderSpecs,
  sliderValues,
} from '@/src/lib/edit-sections';
import { commitSlider, redoHistory, removeSlider, toggleAction, undoHistory } from '@/src/lib/editor/editor-history';
import { editorPreview } from '@/src/lib/editor/editor-preview';
import { THUMBNAIL_HEIGHT, THUMBNAIL_WIDTH } from '@/src/lib/editor/editor-thumbnails';
import { adjustmentKey, isPersonAdjustment, personAdjustmentTargets } from '@/src/lib/skin-adjustments';
import { useTheme } from '@/src/lib/theme';
import {
  actionPreviewRequested,
  blemishApplyRequested,
  cropApplyRequested,
  redEyeAutoRequested,
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
  isPersonControlFocused$,
  openGroupKey,
  openGroupKey$,
  openSliderState,
  openSliderState$,
  areaOutlinesHidden,
} from '@/src/state/edits';
import { canRedo$, canUndo$ } from '@/src/state/history';
import { personOutlinesVisible, personSelection, personSelection$ } from '@/src/state/person-selection';
import { bottomInset$, controlThumbnails$ } from '@/src/state/preview';
import { replayHidden } from '@/src/state/session';
import { VesselView } from '@vessel/react-native';
import { ChevronLeft, Redo2, RotateCcw, ScanFace, Undo2, X } from 'lucide-react-native';
import { useEffect, useMemo, useRef, useState } from 'react';
import { Pressable, ScrollView, StyleSheet, Text, View } from 'react-native';
import { useObservable, useSyncObservable } from 'react-rx';

/** The colors of the bar that picks a color, by the name a control gives its `picker`. */
const barColors = {
  hue: hueColors,
  shade: () => SHADE_COLORS,
  glow: () => GLOW_COLORS,
  'skin-tan': () => TAN_COLORS,
  infrared: () => INFRARED_COLORS,
};

/** The control that is open in the active section, or undefined. */
function useFocusedControl() {
  const sectionKey = useObservable(activeSectionKey$, activeSectionKey.value);
  const key = useObservable(focusedControlKey$, focusedControlKey.value);
  return key ? leafControls(findSection(sectionKey).controls).find(control => control.key === key) : undefined;
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
  const openKey = useObservable(openGroupKey$, openGroupKey.value);
  const groups = openKey ? groupPath(activeSection.controls, openKey) : [];
  useActionPreview();
  const focusedControl = useFocusedControl();

  useEffect(() => {
    const subscriptions = [
      // The live view keeps showing the last tool's result over the photo, so going back or forward has to take that off too.
      undoRequested$.subscribe(() => {
        editorPreview.hide();
        undoHistory();
      }),
      redoRequested$.subscribe(() => {
        editorPreview.hide();
        redoHistory();
      }),
      // Picking a section closes any open slider, including when it is the section already showing, so its controls come back.
      selectSectionRequested$.subscribe(key => {
        focusedControlKey.next(null);
        openGroupKey.next(null);
        activeSectionKey.next(key);
      }),
      // Opening a tool opens its first color; leaving it (back, commit, or a section switch) closes it.
      focusedControlKey$.subscribe(key => {
        openSliderState.next({ key: key ?? '', index: 0 });
      }),
      selectControlRequested$.subscribe(control => {
        if (control.kind === 'action') {
          if (control.tool) {
            // Applying previews at once; removing has nothing to preview, but the same short replay needs no spinner either.
            replayHidden.next(true);
            if (control.repeatable || !appliedActions.value.includes(control.key)) actionPreviewRequested.next(control);
            // The live view is still showing the tool's result over the photo, so taking the tool off has to take that off too.
            else editorPreview.hide();
          }
          toggleAction(control);
        } else if (control.kind === 'group') {
          openGroupKey.next(control.key);
        } else if (
          [
            LIPSTICK_KEY,
            EYELINER_KEY,
            EYESHADOW_KEY,
            BROW_BONE_KEY,
            UNDER_EYE_KEY,
            BLUSH_KEY,
            LIP_LINER_KEY,
            HIGHLIGHTER_KEY,
            CONTOUR_KEY,
            HAIR_COLOR_KEY,
          ].includes(control.key) &&
          missingModels(control.key === HAIR_COLOR_KEY ? HAIR_MODELS : MAKEUP_MODELS).length > 0
        ) {
          // Finding the lips needs the face models, and the hair the hair model; opening the slider without them would do nothing.
          aiModelsRequested.next({
            title: control.key === HAIR_COLOR_KEY ? 'Hair color' : 'Lipstick',
            roles: control.key === HAIR_COLOR_KEY ? HAIR_MODELS : MAKEUP_MODELS,
            onReady: () => selectControlRequested.next(control),
          });
        } else {
          focusedControlKey.next(control.key);
          if (
            control.kind === 'slider' &&
            appliesOnSelect(control) &&
            adjustmentsState.value[control.key] === undefined
          ) {
            if (!isPersonAdjustment(control.key)) {
              commitSlider(control, sliderDefault(control));
            } else {
              // A control for a person starts once it is known who it is for. The people are found a moment after it opens: with
              // one (or none), the starting look goes on everyone, as before; with several, nothing goes on until a person is
              // tapped, and then it goes on that person only.
              personSelection$
                .pipe(
                  filter(state => state.focused && !state.loading),
                  takeUntil(focusedControlKey$.pipe(filter(focused => focused !== control.key))),
                )
                .subscribe(state => {
                  if (state.people.length > 1 && state.selectedId === null) return;
                  if (adjustmentsState.value[adjustmentKey(control.key, state.selectedId)] === undefined) {
                    commitSlider(control, sliderDefault(control));
                  }
                });
            }
          }
        }
      }),
      sliderCommitRequested$.subscribe(({ control, value }) => commitSlider(control, value)),
    ];
    return () => subscriptions.forEach(subscription => subscription.unsubscribe());
  }, []);

  return (
    <View
      style={[styles.panel, { paddingBottom: bottomInset + 12 }]}
      onLayout={event => __DEV__ && console.debug('[panel] height', event.nativeEvent.layout.height, 'bottom inset', bottomInset)}
    >
      <BlemishToolController />
      <RedEyeToolController />
      <PersonSelectionToolController />
      <CropToolController />
      <View style={styles.history}>
        <View style={styles.breadcrumbs}>
          {[
            { key: null, label: activeSection.label, open: null },
            ...groups.map(group => ({ key: group.key, label: group.label, open: group.key })),
            ...(focusedControl ? [{ key: focusedControl.key, label: focusedControl.label, open: undefined }] : []),
          ].map((crumb, index, all) => (
              <Pressable
                key={crumb.key ?? 'section'}
                onPress={() => crumb.open !== undefined && openGroupKey.next(crumb.open)}
                hitSlop={8}
              >
                <Text numberOfLines={1} style={index === all.length - 1 ? styles.crumbCurrent : styles.crumb}>
                  {index > 0 ? '› ' : ''}
                  {crumb.label}
                </Text>
              </Pressable>
            ))}
        </View>
        <Pressable style={styles.historyButton} onPress={() => undoRequested.next()} disabled={!canUndo} hitSlop={8}>
          <Undo2 color={canUndo ? colors.textPrimary : colors.textTertiary} size={20} />
        </Pressable>
        <Pressable style={styles.historyButton} onPress={() => redoRequested.next()} disabled={!canRedo} hitSlop={8}>
          <Redo2 color={canRedo ? colors.textPrimary : colors.textTertiary} size={20} />
        </Pressable>
      </View>
      <View
        onLayout={event => __DEV__ && console.debug('[panel] controls height', event.nativeEvent.layout.height)}
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
        ) : focusedControl?.kind === 'tool' && focusedControl.key === RED_EYE_TOOL_KEY ? (
          <RedEyeControls onBack={() => focusedControlKey.next(null)} onAuto={() => redEyeAutoRequested.next()} />
        ) : focusedControl?.kind === 'tool' && focusedControl.key === CROP_TOOL_KEY ? (
          <CropControls onBack={() => focusedControlKey.next(null)} onApply={() => cropApplyRequested.next()} />
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
  const control = useFocusedControl();
  // Each starts from what the subject holds now: the hook's own first value is the one given here, and a control that opens on
  // an empty one would start its sliders from the defaults and keep them.
  const adjustments = useObservable(adjustments$, adjustmentsState.value);
  const selection = useObservable(personSelection$, personSelection.value);
  const isPersonControlFocused = useObservable(isPersonControlFocused$, false);
  const historyVersion = useObservable(historyVersion$, historyVersionState.value);
  // The AI is working while the bar under the photo shows, and finding the faces again then would only start the work over.
  const aiWorking = Object.keys(useObservable(aiProgress$, aiProgress.value)).length > 0;
  const catalog = useObservable(aiModelList$, aiModelList.value);
  const { colors } = useTheme();
  const styles = useMemo(() => createStyles(colors), [colors]);
  const onLiveChange = useSliderPreview(control?.kind === 'slider' ? control : undefined);
  // Drawn areas step aside while a slider moves and come back when it is released.
  const liveChange = (next: number[]) => {
    areaOutlinesHidden.next(true);
    onLiveChange?.(next);
  };
  const preview = useSyncObservable(editorPreview.view, null);
  // The value of every slider as the user has set them, for a control with several: moving one sends all of them.
  const sliderState = useRef<{ resetKey: string; values: number[] }>({ resetKey: '', values: [] });
  // Which slider a control that shows them one at a time (`display: 'tabulated'`) has open.
  const selectedSlider = useObservable(openSliderState$, openSliderState.value);
  const setSelectedSlider = (value: { key: string; index: number }) => openSliderState.next(value);
  // Counts the times the values were set from outside the sliders (a swatch, a reset), so the sliders and the picker are made
  // again with their handles where the values are now.
  const [valueSets, setValueSets] = useState(0);

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
  // Puts every slider of the control back to its default and applies that, leaving the control open. The close button is the one
  // that turns the effect off.
  const resetAll = (
    <Pressable
      onPress={() => {
        specs.forEach((spec, index) => (values[index] = spec.defaultValue));
        setValueSets(sets => sets + 1);
        sliderCommitRequested.next({ control, value: committedValue() });
      }}
      hitSlop={8}
      style={styles.backChip}
      accessibilityRole="button"
      accessibilityLabel={`Reset ${control.label}`}
    >
      <RotateCcw color={colors.textPrimary} size={16} />
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

  const everyone = isPersonControlFocused ? <PersonEveryoneButton /> : null;

  // Finds again what the control's models found, for when the first time went wrong, redoing only what this control uses. The
  // analyses a control uses say which models they have (see `ai-analyses`), and the button is there when one of those models gives
  // masks (the catalog's `kind`), so a model added to the catalog needs nothing here. Until the catalog has been read, it is there.
  const analyses = analysesUsedBy(control);
  const givesMasks =
    analyses.length > 0 &&
    (catalog.length === 0 ||
      catalog.some(
        model =>
          analyses.some(analysis => analysis.roles.includes(model.id)) && model.kinds.some(kind => MASK_KINDS.includes(kind)),
      ));
  const rescan = givesMasks ? (
    <Pressable
      disabled={aiWorking}
      onPress={() => {
        if (aiWorking) return;
        const regenerate = () => analyses.forEach(analysis => analysis.regenerate());
        const required = [...new Set(analyses.flatMap(analysis => analysis.required))];
        if (missingModels(required).length > 0) {
          aiModelsRequested.next({ title: control.label, roles: required, onReady: regenerate });
        } else regenerate();
      }}
      hitSlop={8}
      style={[styles.backChip, aiWorking && { opacity: 0.4 }]}
      accessibilityRole="button"
      accessibilityState={{ disabled: aiWorking }}
      accessibilityLabel="Find the face again"
    >
      <ScanFace color={colors.textPrimary} size={16} />
    </Pressable>
  ) : null;

  // The bar that picks this control's slider, when it has one in place of the plain slider.
  const barKindAt = (index: number) => {
    const picker = Array.isArray(control.picker) ? control.picker[index] : control.picker;
    return picker && picker in barColors ? (picker as keyof typeof barColors) : undefined;
  };
  const barKind = barKindAt(0);

  return (
    <View
      style={styles.sliderPanel}
      onTouchStart={() => {
        if (personSelection.value.focused) personOutlinesVisible.next(false);
      }}
    >
      <View style={styles.sliderStack}>
        {(isMultiSlider(control) || barKind) && (
          // The back and close buttons belong to the control, so with several sliders they get a header of their own and
          // every slider looks the same. A bar has no header of its own either.
          <View style={styles.sliderHeader}>
            {back}
            <View style={styles.sliderHeaderSpacer} />
            {everyone}
            {rescan}
            {isMultiSlider(control) && resetAll}
            {close}
          </View>
        )}
        {tabulated && (
          <ScrollView horizontal showsHorizontalScrollIndicator={false} contentContainerStyle={styles.sliderPills}>
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
          </ScrollView>
        )}
        {specs.map((spec, index) =>
          tabulated && index !== openSlider ? null : barKindAt(index) ? (
            <View key={resetKey + '-' + index}>
              {control.swatches && (
                <ScrollView horizontal showsHorizontalScrollIndicator={false} contentContainerStyle={styles.swatchRow}>
                  {control.swatches.map(swatch => {
                    const chosen = swatch.values.every(
                      (wanted, swatchIndex) => wanted === undefined || Math.abs(values[swatchIndex] - wanted) < 0.005,
                    );
                    return (
                      <Pressable
                        key={swatch.label}
                        style={styles.swatch}
                        accessibilityRole="button"
                        accessibilityLabel={swatch.label}
                        accessibilityState={{ selected: chosen }}
                        onPress={() => {
                          swatch.values.forEach((wanted, swatchIndex) => {
                            if (wanted !== undefined) values[swatchIndex] = wanted;
                          });
                          setValueSets(sets => sets + 1);
                          // Draws the color on the live preview as a drag of the bar does, then commits it.
                          liveChange([...values]);
                          sliderCommitRequested.next({ control, value: committedValue() });
                        }}
                      >
                        <View
                          style={[styles.swatchDot, { backgroundColor: swatch.color }, chosen && styles.swatchChosen]}
                        />
                        <Text numberOfLines={2} style={[styles.swatchLabel, chosen && styles.swatchLabelChosen]}>
                          {swatch.label}
                        </Text>
                      </Pressable>
                    );
                  })}
                </ScrollView>
              )}
              <View style={styles.sliderTrack}>
                <ColorBarPicker
                  key={valueSets + '-' + historyVersion}
                  colors={barColors[barKindAt(index)!]()}
                  value={values[index]}
                  onChange={value => {
                    values[index] = value;
                    liveChange([...values]);
                  }}
                  onRelease={value => {
                    values[index] = value;
                    sliderCommitRequested.next({ control, value: committedValue() });
                  }}
                />
              </View>
            </View>
          ) : (
            <Slider
              key={resetKey + '-' + index + '-' + valueSets}
              // A single slider carries the buttons in its own header.
              leading={!isMultiSlider(control) && index === 0 ? back : undefined}
              trailing={
                !isMultiSlider(control) && index === 0 ? (
                  <>
                    {everyone}
                    {rescan}
                    {close}
                  </>
                ) : undefined
              }
              label={spec.label}
              min={spec.min}
              max={spec.max}
              step={spec.step}
              initialValue={values[index]}
              resetValue={spec.defaultValue}
              triggerType="release"
              onChange={value => {
                values[index] = value;
                liveChange([...values]);
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
        const adjusted = control.kind === 'action' && !control.repeatable && appliedActions.includes(control.key);
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
  const groupKey = useObservable(openGroupKey$, openGroupKey.value);
  const activeSection = findSection(sectionKey);
  const path = groupKey ? groupPath(activeSection.controls, groupKey) : [];
  const group = path[path.length - 1];
  const isApplied = (control: Exclude<EditControl, GroupControl>) =>
    control.kind === 'action'
      ? !control.repeatable && appliedActions.includes(control.key)
      : control.kind === 'slider' &&
        (isPersonAdjustment(control.key)
          ? personAdjustmentTargets(adjustments, control.key).some(target => isSliderApplied(control, target.value))
          : isSliderApplied(control, adjustments[control.key]));

  return (
    <ScrollView
      key={sectionKey + (group?.key ?? '')}
      horizontal
      showsHorizontalScrollIndicator={false}
      contentContainerStyle={styles.chipRow}
    >
      {group && (
        <Pressable style={styles.backChip} onPress={() => openGroupKey.next(path[path.length - 2]?.key ?? null)} hitSlop={8}>
          <ChevronLeft color={colors.textPrimary} size={18} />
        </Pressable>
      )}
      {(group?.controls ?? activeSection.controls).map(control => {
        const adjusted = leafControls([control]).some(isApplied);
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
    // Pinned to the left so the undo and redo buttons stay centered; long trails are cut short rather than reaching them.
    breadcrumbs: { position: 'absolute', left: 16, maxWidth: '35%', flexDirection: 'row', alignItems: 'center', gap: 4 },
    crumb: { color: colors.textTertiary, fontSize: 12 },
    crumbCurrent: { color: colors.textPrimary, fontSize: 12, fontWeight: '600' },
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
    swatchRow: { gap: 8, paddingVertical: 6, paddingHorizontal: 4 },
    swatch: { alignItems: 'center', gap: 4, width: 64 },
    swatchDot: { width: 30, height: 30, borderRadius: 15, borderWidth: 1, borderColor: colors.textTertiary },
    swatchChosen: { borderWidth: 3, borderColor: colors.accent },
    swatchLabel: { color: colors.textSecondary, fontSize: 11, fontWeight: '500', textAlign: 'center' },
    swatchLabelChosen: { color: colors.textPrimary, fontWeight: '700' },
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
    sliderHeaderSpacer: { flex: 1 },
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
