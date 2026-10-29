import type { useSelection } from '@/src/components/selection';
import type { PluginSurface } from '@/src/lib/plugins/plugin-inspector';
import { useTheme } from '@/src/lib/ui/theme';
import {
  pluginEditorControlsError,
  pluginEditorControlsError$,
  pluginEditorInspectorError,
  pluginEditorInspectorError$,
  pluginEditorItems,
  pluginEditorItems$,
  pluginEditorPicked,
  pluginEditorPicked$,
} from '@/src/state/plugin-editor';
import { useMemo } from 'react';
import { ScrollView, StyleSheet, Text } from 'react-native';
import { useObservable } from 'react-rx';
import { InspectorFilePicker, type InspectorFiles } from './inspector-file-picker';
import { InspectorHeader } from './inspector-header';
import { InspectorNumber } from './inspector-number';
import { InspectorSelect, joinChoices } from './inspector-select';
import { InspectorSlider } from './inspector-slider';
import { InspectorSize } from './inspector-size';
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

  return (
    <ScrollView style={styles.tools} contentContainerStyle={styles.toolList}>
      {error ? <Text style={{ color: colors.danger }}>{error}</Text> : null}
      {items.length === 0 && !error ? (
        <Text style={{ color: colors.textSecondary }}>This plugin added no inspector controls.</Text>
      ) : null}
      {items.map(item => {
        if (item.kind === 'header') {
          return <InspectorHeader key={item.key} text={item.label} level={item.level} />;
        }
        if (item.kind === 'file-picker') {
          return (
            <InspectorFilePicker
              key={item.key}
              itemKey={item.key}
              label={item.label}
              multiple={item.multiple}
              files={item.thumbs ? picked[item.key] : undefined}
              selection={selection}
              onFiles={onFiles}
            />
          );
        }
        if (item.kind === 'select') {
          return (
            <InspectorSelect
              key={item.key}
              itemKey={item.key}
              label={item.label}
              choices={joinChoices(item.options)}
              value={item.value}
            />
          );
        }
        if (item.kind === 'toggle') {
          return <InspectorToggle key={item.key} itemKey={item.key} label={item.label} value={item.value} />;
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
            value={item.value}
          />
        );
      })}
    </ScrollView>
  );
}

const styles = StyleSheet.create({
  tools: { width: '100%', flex: 1 },
  toolList: { padding: 12, gap: 12 },
});
