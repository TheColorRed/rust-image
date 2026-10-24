import { filter, takeUntil } from 'rxjs/operators';
import { ColorBarPicker } from '@/src/components/editor/color-bar-picker';
import { RangeSlider } from '@/src/components/range-slider';
import { Slider } from '@/src/components/slider';
import { BlemishControls, BlemishToolController } from '@/src/components/tools/blemish-tool';
import { ColorAreaToolController } from '@/src/components/tools/color-area-tool';
import { CropControls, CropToolController } from '@/src/components/tools/crop-tool';
import { ObjectRemovalControls, ObjectRemovalToolController } from '@/src/components/tools/object-removal-tool';
import { RedEyeControls, RedEyeToolController } from '@/src/components/tools/red-eye-tool';
import { PersonEveryoneButton, PersonSelectionToolController } from '@/src/components/tools/person-selection-tool';
import {
  GLOW_COLORS,
  INFRARED_COLORS,
  NEON_COLORS,
  SHADE_COLORS,
  TAN_COLORS,
  colorAlongBar,
  hueColors,
} from '@/src/lib/ui/color-bars';
import { missingModels } from '@/src/lib/ai/ai-model-availability';
import { MASK_KINDS, analysesUsedBy } from '@/src/lib/ai/ai-analyses';
import { aiModelList, aiModelList$, aiModelsRequested } from '@/src/state/ai-models';
import { pluginControlsVersion$ } from '@/src/state/plugins';
import { aiProgress, aiProgress$ } from '@/src/state/ai-progress';
import { OBJECT_REMOVAL_TOOL_KEY } from '@/src/lib/project/edit-sections/keys';
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
  type PickerControl,
  type MixedControl,
  type SliderControl,
  isSliderApplied,
  leafControls,
  controlsWith,
  mixedParent,
  sliderDefault,
  sliderSpecs,
  sliderValues,
  pillFormatAt,
  pillText,
  rangePillText,
} from '@/src/lib/project/edit-sections';
import { commitSlider, redoHistory, removeSlider, toggleAction, undoHistory } from '@/src/lib/editor/editor-history';
import { editorPreview } from '@/src/lib/editor/editor-preview';
import { THUMBNAIL_HEIGHT, THUMBNAIL_WIDTH } from '@/src/lib/editor/editor-thumbnails';
import { adjustmentKey, isPersonAdjustment, personAdjustmentTargets } from '@/src/lib/beauty/skin-adjustments';
import { useTheme } from '@/src/lib/ui/theme';
import {
  actionPreviewRequested,
  blemishApplyRequested,
  objectRemovalRequested,
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
  sliderResetRequested,
  sliderResetRequested$,
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
import ChevronLeft from 'lucide-react-native/icons/chevron-left';
import Redo2 from 'lucide-react-native/icons/redo-2';
import RotateCcw from 'lucide-react-native/icons/rotate-ccw';
import ScanFace from 'lucide-react-native/icons/scan-face';
import Undo2 from 'lucide-react-native/icons/undo-2';
import X from 'lucide-react-native/icons/x';
import { Fragment, useEffect, useMemo, useRef, useState } from 'react';
import { Image, Pressable, ScrollView, StyleSheet, Text, View } from 'react-native';
import { useObservable, useSyncObservable } from 'react-rx';

/** The colors of the bar that picks a color, by the name a control gives its `picker`. */
const barColors = {
  hue: hueColors,
  shade: () => SHADE_COLORS,
  glow: () => GLOW_COLORS,
  'skin-tan': () => TAN_COLORS,
  infrared: () => INFRARED_COLORS,
  neon: () => NEON_COLORS,
};

/** The control that is open in the active section, or undefined. */
function useFocusedControl() {
  const sectionKey = useObservable(activeSectionKey$, activeSectionKey.value);
  const key = useObservable(focusedControlKey$, focusedControlKey.value);
  return key ? leafControls(findSection(sectionKey).controls).find(control => control.key === key) : undefined;
}

/** The bottom panel: history buttons, the focused control's editor, and the section tabs. */
export function EditorPanel() {
  // A plugin that adds or takes away controls changes what the panel lists, so it reads them again then.
  useObservable(pluginControlsVersion$, 0);
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
  const openTab = useObservable(openSliderState$, openSliderState.value);
  /** The crumb of the focused control: for the slider of a mixed control's table, the name of the tab that is open. */
  const crumbLabel = (focused: NonNullable<typeof focusedControl>) => {
    const table = mixedParent(activeSection.controls, focused.key);
    if (!table || focused.kind !== 'slider' || openTab.key !== focused.key) return focused.label;
    const extra = table.controls.find(inner => inner.kind === 'picker' && inner.key === openTab.extra);
    if (extra) return extra.label;
    const specs = sliderSpecs(focused);
    return specs[Math.min(openTab.index, specs.length - 1)]?.label ?? focused.label;
  };
  const openGroup = groups[groups.length - 1];
  /** Steps out of what is open: the focused control, back to the row it sits in, or else the open group, back to its parent. */
  const goBack = () => {
    if (focusedControl) {
      focusedControlKey.next(null);
      const table = mixedParent(activeSection.controls, focusedControl.key);
      // A table (a mixed control of only sliders and pickers) closes with its slider, back to the row it sits in. A control in a
      // row of other controls goes back to that row, which is still open.
      if (table?.controls.every(inner => inner.kind === 'slider' || inner.kind === 'picker')) {
        const path = groupPath(activeSection.controls, table.key);
        openGroupKey.next(path.length > 1 ? path[path.length - 2].key : null);
      }
    } else openGroupKey.next(groups[groups.length - 2]?.key ?? null);
  };
  /** Turns off what is open: the focused slider, or every slider of the open mixed control. Undefined when neither is open. */
  const clearOpen =
    focusedControl?.kind === 'slider'
      ? () => {
          editorPreview.hide();
          removeSlider(focusedControl);
          goBack();
        }
      : !focusedControl && openGroup?.kind === 'mixed'
        ? () => {
            editorPreview.hide();
            leafControls([openGroup]).forEach(control => control.kind === 'slider' && removeSlider(control));
            goBack();
          }
        : undefined;
  /**
   * Puts every slider of the open mixed control back to its default and leaves the effect on, as the reset button beside a multi-slider
   * control's close button does for its own sliders. Undefined unless the row of tabs of a mixed control is what is open.
   */
  const resetOpen =
    !focusedControl && openGroup?.kind === 'mixed'
      ? () => leafControls([openGroup]).forEach(control => control.kind === 'slider' && commitSlider(control, sliderDefault(control)))
      : undefined;
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
      // Opening a tool opens its first color, or its first picker tab when it has one before the sliders (`after: -1`); leaving it
      // (back, commit, or a section switch) closes it.
      focusedControlKey$.subscribe(key => {
        const first = key
          ? mixedParent(findSection(activeSectionKey.value).controls, key)?.controls.find(
              (inner): inner is PickerControl => inner.kind === 'picker' && inner.after === -1,
            )
          : undefined;
        openSliderState.next({ key: key ?? '', index: 0, extra: first?.key });
      }),
      selectControlRequested$.subscribe(control => {
        // A mixed control with one tool applies with all its sliders at their defaults as soon as it is opened.
        if (control.kind === 'mixed' && control.tool) {
          for (const inner of leafControls([control])) {
            if (inner.kind === 'slider' && appliesOnSelect(inner) && adjustmentsState.value[inner.key] === undefined) {
              commitSlider(inner, sliderDefault(inner));
            }
          }
        }
        if (control.kind === 'action') {
          if (control.tool) {
            // Applying previews at once; removing has nothing to preview, but the same short replay needs no spinner either.
            replayHidden.next(true);
            if (control.repeatable || !appliedActions.value.includes(control.key)) actionPreviewRequested.next(control);
            // The live view is still showing the tool's result over the photo, so taking the tool off has to take that off too.
            else editorPreview.hide();
          }
          toggleAction(control);
        } else if (
          control.kind === 'mixed' &&
          control.controls[0]?.kind === 'slider' &&
          control.controls.every(inner => inner.kind === 'slider' || inner.kind === 'picker')
        ) {
          // Opens on its first slider, whose tabs the other controls of the mixed control join.
          openGroupKey.next(control.key);
          selectControlRequested.next(control.controls[0]);
        } else if (control.kind === 'group' || control.kind === 'mixed' || control.kind === 'picker') {
          openGroupKey.next(control.key);
        } else if (control.kind === 'slider' && control.needsModels && missingModels(control.needsModels.roles).length > 0) {
          // Finding the lips needs the face models, and the hair the hair model; opening the slider without them would do nothing.
          aiModelsRequested.next({
            title: control.needsModels.title,
            roles: control.needsModels.roles,
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
      onLayout={event =>
        __DEV__ && console.debug('[panel] height', event.nativeEvent.layout.height, 'bottom inset', bottomInset)
      }
    >
      <BlemishToolController />
      <RedEyeToolController />
      <ObjectRemovalToolController />
      <ColorAreaToolController />
      <PersonSelectionToolController />
      <CropToolController />
      <View style={styles.history}>
        <View style={styles.breadcrumbs}>
          {(focusedControl || openGroup) && (
            <Pressable onPress={goBack} hitSlop={8} style={[styles.backChip, styles.backGap]} accessibilityLabel="Back">
              <ChevronLeft color={colors.textPrimary} size={18} />
            </Pressable>
          )}
          {[
            { key: null, label: activeSection.label, open: null },
            ...groups.map(group => ({ key: group.key, label: group.label, open: group.key })),
            // The tabs of a mixed control's table are named by the open one, as `Orb 1 › Color`.
            ...(focusedControl
              ? [{ key: focusedControl.key, label: crumbLabel(focusedControl), open: undefined }]
              : []),
          ].map((crumb, index, all) => (
            <Pressable
              key={crumb.key ?? 'section'}
              onPress={() => {
                if (crumb.open === undefined) return;
                // A crumb is a level above the open control, so the control closes with the jump.
                focusedControlKey.next(null);
                openGroupKey.next(crumb.open);
              }}
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
        {clearOpen && (
          <View style={styles.clearButtons}>
            {resetOpen && (
              <Pressable
                onPress={resetOpen}
                hitSlop={8}
                style={styles.backChip}
                accessibilityRole="button"
                accessibilityLabel={`Reset ${openGroup?.label}`}
              >
                <RotateCcw color={colors.textPrimary} size={16} />
              </Pressable>
            )}
            {focusedControl?.kind === 'slider' && isMultiSlider(focusedControl) && (
              <Pressable
                onPress={() => sliderResetRequested.next()}
                hitSlop={8}
                style={styles.backChip}
                accessibilityRole="button"
                accessibilityLabel={`Reset ${focusedControl.label}`}
              >
                <RotateCcw color={colors.textPrimary} size={16} />
              </Pressable>
            )}
            <Pressable
              onPress={clearOpen}
              hitSlop={8}
              style={styles.backChip}
              accessibilityRole="button"
              accessibilityLabel={`Remove ${focusedControl?.label ?? openGroup?.label}`}
            >
              <X color={colors.textPrimary} size={16} />
            </Pressable>
          </View>
        )}
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
          <BlemishControls onApply={() => blemishApplyRequested.next()} />
        ) : focusedControl?.kind === 'tool' && focusedControl.key === RED_EYE_TOOL_KEY ? (
          <RedEyeControls onAuto={() => redEyeAutoRequested.next()} />
        ) : focusedControl?.kind === 'tool' && focusedControl.key === OBJECT_REMOVAL_TOOL_KEY ? (
          <ObjectRemovalControls onRemove={() => objectRemovalRequested.next()} />
        ) : focusedControl?.kind === 'tool' && focusedControl.key === CROP_TOOL_KEY ? (
          <CropControls onApply={() => cropApplyRequested.next()} />
        ) : focusedControl?.kind === 'slider' ? (
          <SliderEditor />
        ) : activeSection.previewThumbnails && !openKey ? (
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
  // The picker tab of a mixed control that is open in place of the sliders, by key. It is kept with the open slider so the
  // breadcrumbs can name the tab.
  const extraKey = selectedSlider.key === control?.key ? (selectedSlider.extra ?? null) : null;
  const setExtraKey = (extra: string | null) =>
    openSliderState.next({ key: control?.key ?? '', index: selectedSlider.index, extra: extra ?? undefined });
  const sectionKey = useObservable(activeSectionKey$, 'section-colorization');
  const resetAll = useRef<() => void>(() => {});
  useEffect(() => {
    const subscription = sliderResetRequested$.subscribe(() => resetAll.current());
    return () => subscription.unsubscribe();
  }, []);

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
  const parent = mixedParent(findSection(sectionKey).controls, control.key);
  const pickers =
    parent && control.display === 'tabulated'
      ? parent.controls.filter((inner): inner is PickerControl => inner.kind === 'picker')
      : [];
  // Several sliders, or pickers beside the slider, make tabs.
  const tabulated = control.display === 'tabulated' && (specs.length > 1 || pickers.length > 0);
  const extras = tabulated ? pickers : [];
  const openExtra = extras.find(extra => extra.key === extraKey);
  // A different control starts on its first slider.
  const openSlider = openExtra
    ? -1
    : selectedSlider.key === control.key
      ? Math.min(selectedSlider.index, specs.length - 1)
      : 0;
  const committed = sliderValues(control, adjustments[targetKey]);
  /** What is committed for the control: a number for one slider, an array for several. */
  const committedValue = () => (isMultiSlider(control) ? [...values] : values[0]);

  // The reset button in the panel's top row puts every slider of the control back to its default and applies that, leaving the
  // control open. The close button beside it is the one that turns the effect off.
  resetAll.current = () => {
    specs.forEach((spec, index) => (values[index] = spec.defaultValue));
    setValueSets(sets => sets + 1);
    // The live preview still shows the last frame of a drag, so the default is drawn on it first, as a swatch does, and then committed.
    liveChange([...values]);
    sliderCommitRequested.next({ control, value: committedValue() });
  };

  const everyone = isPersonControlFocused ? <PersonEveryoneButton /> : null;

  // Finds again what the control's models found, for when the first time went wrong, redoing only what this control uses. The
  // analyses a control uses say which models they have (see `ai-analyses`), and the button is there when one of those models gives
  // masks (the catalog's `kind`), so a model added to the catalog needs nothing here. Until the catalog has been read, it is there.
  // The controls worked together with this one (the tabs of Pop Art) share its analyses, so they all show that the work is not done.
  const analyses = [
    ...new Set(controlsWith(findSection(sectionKey).controls, control.key).concat(control).flatMap(analysesUsedBy)),
  ];
  const givesMasks =
    analyses.length > 0 &&
    (catalog.length === 0 ||
      catalog.some(
        model =>
          analyses.some(analysis => analysis.roles.includes(model.id)) &&
          model.kinds.some(kind => MASK_KINDS.includes(kind)),
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
    const picker = Array.isArray(control.picker) ? control.picker[specs[index]?.entry ?? index] : control.picker;
    return picker === 'swatches' || (picker && picker in barColors)
      ? (picker as keyof typeof barColors | 'swatches')
      : undefined;
  };
  // The color of the named color the committed values are at, for a control that picks only from its swatches, as `[red, green, blue]`.
  const swatchColorOf = (at: number[]) => {
    const chosen = control.swatches?.find(swatch =>
      swatch.values.every((wanted, index) => wanted === undefined || Math.abs(at[index] - wanted) < 0.005),
    );
    return chosen && [1, 3, 5].map(start => parseInt(chosen.color.slice(start, start + 2), 16));
  };
  const barKind = barKindAt(0);

  const extraTab = (extra: PickerControl) => (
    <Pressable
      key={extra.key}
      style={[styles.sliderPill, extra.key === openExtra?.key && styles.sliderPillOpen]}
      onPress={() => setExtraKey(extra.key)}
      accessibilityRole="tab"
      accessibilityState={{ selected: extra.key === openExtra?.key }}
    >
      <Text style={[styles.sliderPillText, extra.key === openExtra?.key && styles.sliderPillTextOpen]}>
        {extra.label}
      </Text>
    </Pressable>
  );
  const extraAfter = (index: number) => extras.filter(extra => extra.after === index).map(extraTab);

  return (
    <View
      style={styles.sliderPanel}
      onTouchStart={() => {
        if (personSelection.value.focused) personOutlinesVisible.next(false);
      }}
    >
      <View style={styles.sliderStack}>
        {(isMultiSlider(control) || barKind) && (
          // The buttons that belong to the control get a header of their own with several sliders, so every slider looks the
          // same. A bar has no header of its own either.
          <View style={styles.sliderHeader}>
            <View style={styles.sliderHeaderSpacer} />
            {everyone}
            {rescan}
          </View>
        )}
        {tabulated && (
          <ScrollView horizontal showsHorizontalScrollIndicator={false} contentContainerStyle={styles.sliderPills}>
            {extraAfter(-1)}
            {specs.map((spec, index) => {
              // The high thumb of a range slider shares the tab of the low one.
              if (spec.range === 'high') return null;
              const high = spec.range === 'low' ? specs[index + 1] : undefined;
              // A slider with a color bar shows the color it is at, rather than a number a person cannot read as a color.
              const pickerKind = barKindAt(index);
              const pickerColor =
                pickerKind === 'swatches'
                  ? swatchColorOf(committed)
                  : pickerKind
                    ? colorAlongBar(barColors[pickerKind](), (committed[index] - spec.min) / (spec.max - spec.min))
                    : undefined;
              return (
                <Fragment key={index}>
                  <Pressable
                    style={[styles.sliderPill, index === openSlider && styles.sliderPillOpen]}
                    onPress={() => {
                      // The picker may have saved a value since the sliders last read theirs, so they read them again.
                      if (openExtra) sliderState.current.resetKey = '';
                      setSelectedSlider({ key: control.key, index });
                    }}
                    accessibilityRole="tab"
                    accessibilityState={{ selected: index === openSlider }}
                  >
                    <Text style={[styles.sliderPillText, index === openSlider && styles.sliderPillTextOpen]}>
                      {pickerKind
                        ? spec.label
                        : high
                          ? rangePillText(spec, committed[index], high, committed[index + 1])
                          : pillText(spec, committed[index], pillFormatAt(control, spec.entry))}
                    </Text>
                    {pickerColor ? (
                      <View style={[styles.sliderPillSwatch, { backgroundColor: `rgb(${pickerColor.join(', ')})` }]} />
                    ) : (
                      /* A dot marks a slider that is off its default, as the committed value has it. */
                      (committed[index] !== spec.defaultValue ||
                        (high && committed[index + 1] !== high.defaultValue)) && (
                        <View style={[styles.sliderPillDot, index === openSlider && styles.sliderPillDotOpen]} />
                      )
                    )}
                  </Pressable>
                  {extraAfter(index)}
                </Fragment>
              );
            })}
            {extras.filter(extra => extra.after === undefined).map(extraTab)}
          </ScrollView>
        )}

        {openExtra && <PickerItems key={openExtra.key} picker={openExtra} />}
        {specs.map((spec, index) =>
          spec.range === 'high' || (tabulated && index !== openSlider) ? null : spec.range === 'low' ? (
            <RangeSlider
              key={resetKey + '-' + index + '-' + valueSets}
              label={spec.label}
              min={[spec.min, specs[index + 1].min]}
              max={[spec.max, specs[index + 1].max]}
              step={[spec.step, specs[index + 1].step]}
              initialValue={[values[index], values[index + 1]]}
              resetValue={[spec.defaultValue, specs[index + 1].defaultValue]}
              valueText={spec.valueText}
              onChange={([low, high]) => {
                values[index] = low;
                values[index + 1] = high;
                liveChange([...values]);
              }}
              onTrigger={([low, high]) => {
                values[index] = low;
                values[index + 1] = high;
                sliderCommitRequested.next({ control, value: committedValue() });
              }}
            />
          ) : barKindAt(index) ? (
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
              {barKindAt(index) !== 'swatches' && (
                <View style={styles.sliderTrack}>
                  <ColorBarPicker
                    key={valueSets + '-' + historyVersion}
                    colors={barColors[barKindAt(index) as keyof typeof barColors]()}
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
              )}
            </View>
          ) : (
            <Slider
              key={resetKey + '-' + index + '-' + valueSets}
              // A single slider carries the buttons in its own header.
              trailing={
                !isMultiSlider(control) && index === 0 ? (
                  <>
                    {everyone}
                    {rescan}
                  </>
                ) : undefined
              }
              label={spec.label}
              min={spec.min}
              max={spec.max}
              step={spec.step}
              initialValue={values[index]}
              valueText={spec.valueText}
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
                // As with the reset button above: the default is drawn on the live preview too, which shows the last frame of a drag.
                liveChange([...values]);
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
        if (__DEV__ && thumbnail) console.debug('[panel] card drawn by React', control.key);
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

/** The items of an open picker, shown as pictures in the row. Each tap calls that item's own `onTap`. */
function PickerItems({ picker }: { picker: PickerControl }) {
  const { colors } = useTheme();
  const styles = useMemo(() => createStyles(colors), [colors]);
  const controlThumbnails = useObservable(controlThumbnails$, {});
  const adjustments = useObservable(adjustments$, adjustmentsState.value);
  const sectionKey = useObservable(activeSectionKey$, 'section-colorization');
  const [marked, setMarked] = useState<string[]>([]);
  // With a `commitTo`, the choice is a value saved on a slider control, so the marked item is read from there.
  const target = picker.commitTo;
  const slider = target
    ? leafControls(findSection(sectionKey).controls).find(
        (control): control is SliderControl => control.kind === 'slider' && control.key === target.slider,
      )
    : undefined;
  const saved = slider && target ? (sliderValues(slider, adjustments[slider.key])[target.index] ?? 0) : undefined;
  return (
    <ScrollView horizontal showsHorizontalScrollIndicator={false} contentContainerStyle={styles.thumbnailRow}>
      {picker.items.map(item => {
        const isMarked = slider ? item.value === saved : marked.includes(item.key);
        const thumbnail = controlThumbnails[item.key];
        return (
          <Pressable
            key={item.key}
            style={styles.thumbnailButton}
            accessibilityRole="button"
            accessibilityLabel={item.label}
            accessibilityState={{ selected: isMarked }}
            onPress={() => {
              setMarked(current =>
                picker.selection === 'one'
                  ? [item.key]
                  : current.includes(item.key)
                    ? current.filter(key => key !== item.key)
                    : [...current, item.key],
              );
              if (slider && target && item.value !== undefined) {
                const values = sliderValues(slider, adjustments[slider.key]);
                values[target.index] = item.value;
                // The live view still shows the last slider drag over the photo, which would cover the new shape.
                editorPreview.hide();
                commitSlider(slider, values);
              }
              item.onTap?.();
            }}
          >
            <View style={[styles.thumbnailImage, isMarked && styles.thumbnailApplied]}>
              {item.picture.kind === 'asset' ? (
                <Image source={item.picture.source} style={styles.thumbnailCanvas} resizeMode="contain" />
              ) : VesselView && thumbnail ? (
                <VesselView source={thumbnail} style={styles.thumbnailCanvas} pointerEvents="none" />
              ) : (
                <Text style={styles.thumbnailPlaceholder}>{item.label}</Text>
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
  const isApplied = (control: Exclude<EditControl, GroupControl | MixedControl>) =>
    control.kind === 'action'
      ? !control.repeatable && appliedActions.includes(control.key)
      : control.kind === 'slider' &&
        (isPersonAdjustment(control.key)
          ? personAdjustmentTargets(adjustments, control.key).some(target => isSliderApplied(control, target.value))
          : isSliderApplied(control, adjustments[control.key]));

  // Inside a mixed control the effect is already on, so a part is marked only when it was moved off its defaults.
  const isChanged = (control: Exclude<EditControl, GroupControl | MixedControl>) =>
    control.kind === 'slider' &&
    sliderValues(control, adjustments[control.key]).some(
      (value, index) => index < sliderSpecs(control).length && value !== sliderSpecs(control)[index].defaultValue,
    );

  if (group?.kind === 'picker') {
    return <PickerItems key={sectionKey + group.key} picker={group} />;
  }
  const rowControls = group ? group.controls : activeSection.controls;

  return (
    <ScrollView
      key={sectionKey + (group?.key ?? '')}
      horizontal
      showsHorizontalScrollIndicator={false}
      contentContainerStyle={styles.chipRow}
    >
      {rowControls.map(control => {
        // A blue chip is a one-shot action that is on. Any other chip that holds changes, itself or in the controls inside it,
        // carries a blue dot instead.
        const selected = control.kind === 'action' && isApplied(control);
        const changed = !selected && leafControls([control]).some(group?.kind === 'mixed' ? isChanged : isApplied);
        return (
          <Pressable
            key={control.key}
            style={[styles.chip, selected && styles.chipApplied]}
            onPress={() => selectControlRequested.next(control)}
          >
            <Text style={[styles.chipText, selected && styles.chipTextApplied]}>{control.label}</Text>
            {changed && <View style={styles.sliderPillDot} />}
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
    breadcrumbs: {
      position: 'absolute',
      left: 16,
      maxWidth: '35%',
      flexDirection: 'row',
      alignItems: 'center',
      gap: 4,
    },
    backGap: { marginRight: 10 },
    clearButtons: { position: 'absolute', right: 16, flexDirection: 'row', alignItems: 'center', gap: 10 },
    crumb: { color: colors.textTertiary, fontSize: 12 },
    crumbCurrent: { color: colors.textPrimary, fontSize: 12, fontWeight: '600' },
    controls: { justifyContent: 'center' },
    fixedControls: { height: 96 },
    // An open slider control is as tall as its sliders, so several of them do not spill over the buttons above and the tabs below.
    sliderControls: { minHeight: 96 },
    thumbnailControls: { height: THUMBNAIL_HEIGHT + 16 },
    chipRow: { paddingHorizontal: 16, gap: 8, alignItems: 'center' },
    chip: { flexDirection: 'row', alignItems: 'center', gap: 6, paddingHorizontal: 14, paddingVertical: 8, borderRadius: 16, backgroundColor: colors.chip },
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
    sliderPillSwatch: { width: 12, height: 12, borderRadius: 6, borderWidth: 1, borderColor: colors.textTertiary },
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
