import { FilePicker, type PickedImage } from '@/src/components/file-picker';
import type { PluginFilePickerSource, PluginInspectorItem } from '@/src/lib/plugins/plugin-inspector';
import { pluginEditorItems, pluginEditorPicked, pluginEditorProjectId, pluginEditorProjectId$ } from '@/src/state/plugin-editor';
import { memo } from 'react';
import { useObservable } from 'react-rx';
import { StyleSheet } from 'react-native';
import { InspectorRow } from './inspector-row';

export type InspectorFiles = (item: Extract<PluginInspectorItem, { kind: 'file-picker' }>, files: PickedImage[]) => void;

type Props = {
  /** Names the picker among the plugin's controls, which is what its images are kept under and how its current settings are found. */
  itemKey: string;
  label: string;
  source: PluginFilePickerSource;
  multiple: boolean;
  /** The picker's images changed: more were picked, or some were removed. Gets the picker and the full list. */
  onFiles: InspectorFiles;
};

/**
 * The button of a file picker a plugin put in its inspector. The pictures it holds are shown by `ImageThumbs`, which the inspector puts
 * under it. It is drawn again only when its own settings change.
 */
export const InspectorFilePicker = memo(function InspectorFilePicker({ itemKey, label, source, multiple, onFiles }: Props) {
  // The workspace's own project is left out of the project grid, since a picture cannot be picked into the project it is in.
  const projectId = useObservable(pluginEditorProjectId$, pluginEditorProjectId.value);
  return (
    <InspectorRow label={label}>
      <FilePicker
        title={label}
        hideTitle
        source={source}
        excludeProjectId={projectId}
        multiple={multiple}
        variant="inline"
        onPick={added => {
          const item = pluginEditorItems.value.find(each => each.key === itemKey);
          if (item?.kind === 'file-picker') onFiles(item, [...(pluginEditorPicked.value[itemKey] ?? []), ...added]);
        }}
        showPreview={false}
        style={styles.picker}
      />
    </InspectorRow>
  );
});

const styles = StyleSheet.create({
  picker: { width: '100%' },
});
