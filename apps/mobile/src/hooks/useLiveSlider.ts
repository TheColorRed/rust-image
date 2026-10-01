import { type AbraImage } from '@alakazam/mobile';
import { useEffect, useMemo, useRef, useState } from 'react';
import { PixelRatio } from 'react-native';
import { useObservable } from 'react-rx';
import { useLivePreview } from '@/src/hooks/useLivePreview';
import { type SliderControl } from '@/src/lib/edit-sections';
import { canChainLive, liveChain, renderLivePreviewBase } from '@/src/lib/editor-replay';
import { appliedActions$, adjustments$ } from '@/src/state/edits';
import { previewBox$ } from '@/src/state/gestures';
import { LIVE_SURFACE_ID } from '@/src/components/editor/abra-live-view';
import { liveFrame, liveSurface } from '@/src/state/preview';

const LIVE_SCALE = 1;

/**
 * Live GPU preview for a slider drag. Returns the change handler to give the slider, or undefined when the control has no
 * live form. Frames are shown over the editor preview until the released value is committed and replayed.
 *
 * When every applied control has a live form, the preview starts from the untouched edit base and the whole stack runs as
 * one GPU effect chain, in the stack's own order. A drag only changes its own effect in the chain, so nothing is
 * re-rendered on the CPU. When some applied control is CPU-only, the rest of the stack is rendered on the CPU once and
 * only the dragged slider's effect runs on the GPU on top of it.
 */
export function useLiveSlider(p_control: SliderControl | undefined): ((value: number) => void) | undefined {
  const box = useObservable(previewBox$, null);
  const adjustments = useObservable(adjustments$, {});
  const actions = useObservable(appliedActions$, []);
  const [rest, setRest] = useState<AbraImage | null>(null);
  const live = p_control?.live;
  const key = p_control?.key;

  // Everything except this slider changes what a drag should start from; its own value does not.
  const others = useMemo(() => JSON.stringify([key, actions, { ...adjustments, [key ?? '']: 0 }]), [key, actions, adjustments]);
  const chained = useMemo(() => (key ? canChainLive(key) : false), [key, actions, adjustments]);
  // A chain starts from the untouched base, so only the CPU fallback has to start over when something else changes.
  const restKey = chained ? 'chain' : others;
  /** The `others` the preview's effect chain was last built for. */
  const builtFor = useRef<string | null>(null);

  useEffect(() => {
    if (!live || !key) return;
    const image = renderLivePreviewBase(key);
    builtFor.current = null;
    setRest(image);
    return () => {
      setRest(null);
      image?.uniffiDestroy();
    };
  }, [live, key, restKey]);

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

  if (!live || !key || !box) return undefined;
  return value => {
    if (!chained) return preview.apply(live(value), key);
    // The chain is built on the first change, not when the slider opens, so opening one does not swap in a preview frame.
    if (builtFor.current !== others || !preview.has(key)) {
      preview.clear();
      for (const step of liveChain(key, value)) preview.apply(step.effect, step.key);
      builtFor.current = others;
    } else {
      preview.apply(live(value), key);
    }
  };
}
