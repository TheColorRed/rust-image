import { FilePicker, type PickedImage } from '@/src/components/file-picker';
import type { useSelection } from '@/src/components/selection';
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
  /** The images picked, shown as thumbnails. `undefined` when the plugin asked for no thumbnails. */
  files: PickedImage[] | undefined;
  /** Lets the thumbnails be selected. The screen owns it so it can show the `SelectionBar`. */
  selection: ReturnType<typeof useSelection>;
  /** The picker's images changed: more were picked, or some were removed. Gets the picker and the full list. */
  onFiles: InspectorFiles;
};

/**
 * A file picker a plugin put in its inspector. It is drawn again only when its own settings or images change, or what is selected: the
 * selection is made again on every render of the screen, but its functions only ever set state, so only what it holds matters.
 */
export const InspectorFilePicker = memo(function InspectorFilePicker({
  itemKey,
  label,
  source,
  multiple,
  files,
  selection,
  onFiles,
}: Props) {
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
      thumbs={files}
      variant="inline"
      selection={selection}
      onPick={added => {
        const item = pluginEditorItems.value.find(each => each.key === itemKey);
        if (item?.kind === 'file-picker') onFiles(item, [...(pluginEditorPicked.value[itemKey] ?? []), ...added]);
      }}
      showPreview={false}
      style={styles.picker}
    />
    </InspectorRow>
  );
}, (before, after) =>
  before.itemKey === after.itemKey &&
  before.label === after.label &&
  before.source === after.source &&
  before.multiple === after.multiple &&
  before.files === after.files &&
  before.onFiles === after.onFiles &&
  before.selection.selected === after.selection.selected &&
  before.selection.previewIds === after.selection.previewIds);

const styles = StyleSheet.create({
  picker: { width: '100%' },
});
