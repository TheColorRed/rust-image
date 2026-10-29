import { previewViewport } from '@/src/lib/editor/preview-coordinates';
import { useOwning } from '@/src/lib/native/native-handles';
import { pluginEditorCanvas, pluginEditorRedraw$ } from '@/src/state/plugin-editor';
import type { CanvasLike } from '@abra/core/canvas';
import { ImagePreview } from '@alakazam/mobile';
import { VesselView } from '@vessel/react-native';
import { useEffect, useRef, useState } from 'react';
import { StyleSheet, View, type LayoutChangeEvent } from 'react-native';
import { auditTime } from 'rxjs';

type Open = { preview: ImagePreview; canvas: CanvasLike };

/**
 * Shows the canvas a plugin's `previewCanvas` returned, contained in the space it is given, the way the editor shows a photo. The
 * canvas is shared, not copied: the Vessel engine composes it on its own thread whenever it has changed, so a plugin that changes its
 * layers (a slider) and says so through `pluginEditorRedraw` draws nothing on the JS thread. The plugin owns the canvas and this
 * only looks at it.
 */
export function WorkspacePreview({ canvas }: { canvas: CanvasLike | null }) {
  const [box, setBox] = useState<{ width: number; height: number } | null>(null);
  const alive = useRef<Open | null>(null);
  const shown = useRef(canvas);
  shown.current = canvas;

  const open = useOwning(
    keep => {
      const current = shown.current;
      if (!current || !box) return null;
      const preview = keep(ImagePreview.fromCanvas(current));
      preview.setViewport(...previewViewport({ width: current.width(), height: current.height() }, box, 1, { x: 0, y: 0 }));
      return { preview, canvas: current };
    },
    [canvas, box?.width, box?.height],
    alive,
  );

  // Looking again at the canvas: the plugin changed it in place. The canvas can have a new size, so the view is laid out again too.
  useEffect(() => {
    const redraws = pluginEditorRedraw$.pipe(auditTime(16)).subscribe(() => {
      const current = alive.current;
      // A plugin that returns a new canvas frees the one before it, and React draws the new one a moment later, so until then the open
      // preview holds a canvas that is gone. The state already has the new one, so it is the state that says whether the open one is
      // still the canvas on show; its size is not asked of a canvas that is not. The new preview is laid out when it opens.
      if (!current || !box || current.canvas !== pluginEditorCanvas.value) return;
      current.preview.setViewport(
        ...previewViewport({ width: current.canvas.width(), height: current.canvas.height() }, box, 1, { x: 0, y: 0 }),
      );
      current.preview.refresh();
    });
    return () => redraws.unsubscribe();
  }, [box]);

  const onLayout = (event: LayoutChangeEvent) => {
    const { width, height } = event.nativeEvent.layout;
    setBox(previous => (previous && previous.width === width && previous.height === height ? previous : { width, height }));
  };

  return (
    <View style={styles.area} onLayout={onLayout}>
      {open && box && VesselView ? (
        <VesselView source={open.preview} pointerEvents="none" style={{ width: box.width, height: box.height }} />
      ) : null}
    </View>
  );
}

const styles = StyleSheet.create({
  area: { ...StyleSheet.absoluteFill },
});
