import type { PickedImage } from '@/src/components/file-picker';
import { ImageThumbs } from '@/src/components/image-thumbs';
import { pluginEditorItems, pluginEditorRebuild } from '@/src/state/plugin-editor';
import { memo } from 'react';
import { InspectorRow } from './inspector-row';

/**
 * Pictures a plugin put in its inspector to choose from. A tap on one tells the plugin which, and the editor runs it again. Like the other
 * controls it takes plain values and finds the plugin's current `onPick` in the state, because the plugin makes a new one each time.
 */
export const InspectorPictures = memo(function InspectorPictures({
  itemKey,
  label,
  files,
  rows,
}: {
  /** Names the control among the plugin's controls, which is how its current `onPick` is found. */
  itemKey: string;
  label: string;
  files: PickedImage[];
  rows: number | 'all';
}) {
  return (
    <InspectorRow label={label} stacked>
      <ImageThumbs
        files={files}
        rows={rows}
        onPress={(_, index) => {
          const item = pluginEditorItems.value.find(each => each.key === itemKey);
          if (item?.kind === 'pictures') item.onPick?.(index);
          pluginEditorRebuild.next();
        }}
      />
    </InspectorRow>
  );
});
