import { type AbraImage } from '@alakazam/mobile';
import { useEffect, useState } from 'react';
import { PixelRatio } from 'react-native';
import { useObservable } from 'react-rx';
import { useLivePreview } from '@/src/hooks/useLivePreview';
import { type SliderControl } from '@/src/lib/edit-sections';
import { renderEditStackWithoutSlider } from '@/src/lib/editor-replay';
import { appliedActions$, adjustments$ } from '@/src/state/edits';
import { previewBox$ } from '@/src/state/gestures';
import { LIVE_SURFACE_ID } from '@/src/components/editor/abra-live-view';
import { liveFrame, liveSurface } from '@/src/state/preview';

const LIVE_SCALE = 1;

/**
 * Live GPU preview for a slider drag. Returns the change handler to give the slider, or undefined when the control has no
 * live form. Frames are shown over the editor preview until the released value is committed and replayed.
 */
export function useLiveSlider(p_control: SliderControl | undefined): ((value: number) => void) | undefined {
  const box = useObservable(previewBox$, null);
  const adjustments = useObservable(adjustments$, {});
  const actions = useObservable(appliedActions$, []);
  const [rest, setRest] = useState<AbraImage | null>(null);
  const live = p_control?.live;
  const key = p_control?.key;

  // Everything except this slider changes what a drag should start from; its own value does not.
  const others = JSON.stringify([actions, { ...adjustments, [key ?? '']: 0 }]);

  useEffect(() => {
    if (!live || !key) return;
    const image = renderEditStackWithoutSlider(key);
    setRest(image);
    return () => {
      setRest(null);
      image?.uniffiDestroy();
    };
  }, [live, key, others]);

  // Where the native surface is available frames never reach JS; otherwise they are copied out on the JS thread, so LIVE_SCALE can be lowered.
  const ratio = PixelRatio.get() * LIVE_SCALE;
  const preview = useLivePreview(rest, Math.round((box?.width ?? 0) * ratio), Math.round((box?.height ?? 0) * ratio), 0, LIVE_SURFACE_ID);

  useEffect(() => {
    liveFrame.next(preview.frame);
    return () => liveFrame.next(null);
  }, [preview.frame]);

  useEffect(() => {
    liveSurface.next(preview.surface);
    return () => liveSurface.next(null);
  }, [preview.surface]);

  if (!live || !box) return undefined;
  return value => preview.apply(live(value));
}
