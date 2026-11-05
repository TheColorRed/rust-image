import { ImageThumbs } from '@/src/components/image-thumbs';
import type { useSelection } from '@/src/components/selection';
import type { PluginSurface } from '@/src/lib/plugins/plugin-inspector';
import { useTheme } from '@/src/lib/ui/theme';
import {
  pluginEditorControlsError,
  pluginEditorControlsError$,
  pluginEditorGroups,
  pluginEditorGroups$,
  pluginEditorInspectorError,
  pluginEditorInspectorError$,
  pluginEditorItems,
  pluginEditorItems$,
  pluginEditorPicked,
  pluginEditorPicked$,
} from '@/src/state/plugin-editor';
import { Fragment, useMemo } from 'react';
import { ScrollView, StyleSheet, Text } from 'react-native';
import { useObservable } from 'react-rx';
import { InspectorColor } from './inspector-color';
import { InspectorFilePicker, type InspectorFiles } from './inspector-file-picker';
import { InspectorFontPicker } from './inspector-font-picker';
import { InspectorGroup } from './inspector-group';
import { InspectorCurve } from './inspector-curve';
import { InspectorGradient } from './inspector-gradient';
import { InspectorHeader } from './inspector-header';
import { InspectorNumber } from './inspector-number';
import { InspectorPictures } from './inspector-pictures';
import { InspectorSelect, joinChoices } from './inspector-select';
import { InspectorSlider } from './inspector-slider';
import { InspectorSize } from './inspector-size';
import { InspectorSubmenu } from './inspector-submenu';
import { InspectorText } from './inspector-text';
import { InspectorToggle } from './inspector-toggle';

/** The controls a plugin made for one surface (`inspectorGUI` for the inspector, `controlsGUI` for the controls bar): headings, file pickers and sliders, in the order it made them. */
export function Inspector({
  surface,
  selection,
  onFiles,
}: {
  /** Which of the plugin's controls to show. */
  surface: PluginSurface;
  /** Lets the thumbnails of a file picker be selected. The screen owns it so it can show the `SelectionBar`. */
  selection: ReturnType<typeof useSelection>;
  /** A file picker's images changed: more were picked, or some were removed. Gets the picker and the full list. */
  onFiles: InspectorFiles;
}) {
  const { colors } = useTheme();
  const everything = useObservable(pluginEditorItems$, pluginEditorItems.value);
  const items = useMemo(() => everything.filter(item => item.surface === surface), [everything, surface]);
  const inspectorError = useObservable(pluginEditorInspectorError$, pluginEditorInspectorError.value);
  const controlsError = useObservable(pluginEditorControlsError$, pluginEditorControlsError.value);
  const error = surface === 'inspector' ? inspectorError : controlsError;
  const picked = useObservable(pluginEditorPicked$, pluginEditorPicked.value);
  const groups = useObservable(pluginEditorGroups$, pluginEditorGroups.value);
  // Whether the controls being listed are showing: those before the first group always are.
  let shown = true;

  return (
    <ScrollView style={styles.tools} contentContainerStyle={styles.toolList}>
      {error ? <Text style={{ color: colors.danger }}>{error}</Text> : null}
      {items.length === 0 && !error ? (
        <Text style={{ color: colors.textSecondary }}>This plugin added no inspector controls.</Text>
      ) : null}
      {items.map((item, index) => {
        if (item.kind === 'group') {
          const isOpen = groups[item.key] ?? item.open;
          shown = isOpen;
          return (
            <InspectorGroup
              key={item.key}
              text={item.label}
              open={isOpen}
              first={index === 0}
              itemKey={item.key}
              enabled={item.enabled}
              onFold={() => pluginEditorGroups.next({ ...pluginEditorGroups.value, [item.key]: !isOpen })}
            />
          );
        }
        // The controls after a folded group are hidden up to the next group.
        if (!shown) return null;
        if (item.kind === 'header') {
          return <InspectorHeader key={item.key} text={item.label} level={item.level} first={index === 0} onBack={item.back} />;
        }
        if (item.kind === 'submenu') {
          return <InspectorSubmenu key={item.key} itemKey={item.key} text={item.label} />;
        }
        if (item.kind === 'file-picker') {
          return (
            <Fragment key={item.key}>
              <InspectorFilePicker
                itemKey={item.key}
                label={item.label}
                source={item.source}
                multiple={item.multiple}
                onFiles={onFiles}
              />
              {item.thumbs && picked[item.key]?.length ? <ImageThumbs files={picked[item.key]} selection={selection} /> : null}
            </Fragment>
          );
        }
        if (item.kind === 'pictures') {
          return <InspectorPictures key={item.key} itemKey={item.key} label={item.label} files={item.files} rows={item.rows} />;
        }
        if (item.kind === 'select') {
          return (
            <InspectorSelect
              key={item.key}
              itemKey={item.key}
              label={item.label}
              choices={joinChoices(item.options)}
              value={item.value}
              momentary={item.momentary}
            />
          );
        }
        if (item.kind === 'text') {
          return <InspectorText key={item.key} itemKey={item.key} label={item.label} value={item.value} placeholder={item.placeholder} />;
        }
        if (item.kind === 'font-picker') {
          return <InspectorFontPicker key={item.key} itemKey={item.key} label={item.label} value={item.value} />;
        }
        if (item.kind === 'toggle') {
          return <InspectorToggle key={item.key} itemKey={item.key} label={item.label} value={item.value} />;
        }
        if (item.kind === 'color') {
          return (
            <InspectorColor
              key={item.key}
              itemKey={item.key}
              label={item.label}
              value={item.value}
              alpha={item.alpha}
              title={item.title}
              swatches={item.swatches}
            />
          );
        }
        if (item.kind === 'curve') {
          return (
            <InspectorCurve
              key={item.key}
              itemKey={item.key}
              label={item.label}
              value={item.value}
              min={item.min}
              max={item.max}
              title={item.title}
              presets={item.presets}
            />
          );
        }
        if (item.kind === 'gradient') {
          return <InspectorGradient key={item.key} itemKey={item.key} label={item.label} value={item.value} title={item.title} swatches={item.swatches} />;
        }
        if (item.kind === 'number') {
          return (
            <InspectorNumber
              key={item.key}
              itemKey={item.key}
              label={item.label}
              value={item.value}
              min={item.min}
              max={item.max}
              step={item.step}
              reset={item.reset}
            />
          );
        }
        if (item.kind === 'size') {
          return (
            <InspectorSize
              key={item.key}
              itemKey={item.key}
              label={item.label}
              labels={item.labels}
              reset={item.reset}
              x={item.value[0]}
              y={item.value[1]}
              min={item.min}
              max={item.max}
            />
          );
        }
        return (
          <InspectorSlider
            key={item.key}
            itemKey={item.key}
            label={item.label}
            min={item.min}
            max={item.max}
            step={item.step}
            range={item.range}
            multiline={item.multiline}
            reset={item.reset}
            rebuild={item.rebuild}
            value={item.value}
          />
        );
      })}
    </ScrollView>
  );
}

const styles = StyleSheet.create({
  tools: { width: '100%', flex: 1 },
  toolList: { padding: 12, gap: 4 },
});
