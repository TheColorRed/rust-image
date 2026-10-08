import { AbraImage, EffectSpec, Message, type AbraImageLike } from '@alakazam/mobile';
import { useEffect, useRef, useState } from 'react';
import { Button, ScrollView, StyleSheet, Text, useWindowDimensions, View } from 'react-native';
import { FilePicker, type PickedImage } from '@/src/components/file-picker';
import { Slider } from '@/src/components/slider';
import { VesselView } from '@vessel/react-native';
import { useLiveSession } from '@/src/hooks/useLiveSession';

type Operation = { label: string; run: (image: AbraImageLike) => void };

const OPERATIONS: Operation[] = [
  { label: 'Grayscale', run: (image) => image.applyEffect(EffectSpec.Grayscale.new()) },
  { label: 'Invert', run: (image) => image.invert() },
  { label: 'Brightness +40', run: (image) => image.applyEffect(EffectSpec.Brightness.new({ amount: 40 })) },
  { label: 'Auto tone', run: (image) => image.autoTone() },
  { label: 'Blur 8px', run: (image) => image.applyEffect(EffectSpec.GaussianBlur.new({ radius: 8 })) },
  { label: 'Sharpen', run: (image) => image.sharpen() },
];

const PREVIEW_HEIGHT = 320;

/** Converts a `file://` URI to the filesystem path abra reads and writes. */
const toPath = (uri: string) => decodeURIComponent(uri.replace(/^file:\/\//, ''));

export default function AbraScreen() {
  const { width: windowWidth } = useWindowDimensions();
  const previewWidth = windowWidth - 32;
  const [source, setSource] = useState<PickedImage | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  // The working image lives in Rust at full resolution; only screen-sized previews cross into JS.
  const imageRef = useRef<AbraImage | null>(null);
  const [revision, setRevision] = useState(0);
  const session = useLiveSession(source ? imageRef.current : null);
  /** Shows an effect over the preview. */
  const preview = (effect: EffectSpec) => session.send(Message.Show.new([effect]));

  // The JS garbage collector can't see the Rust-side pixels behind a handle, so free them explicitly.
  useEffect(() => () => imageRef.current?.uniffiDestroy(), []);

  function load(asset: PickedImage) {
    try {
      const started = Date.now();
      imageRef.current?.uniffiDestroy();
      imageRef.current = AbraImage.read(toPath(asset.uri)) as AbraImage;
      const image = imageRef.current;
      setSource(asset);
      setRevision((r) => r + 1);
      setStatus(`Loaded ${image.width()}×${image.height()} in ${Date.now() - started} ms`);
    } catch (e: any) {
      setStatus(`Failed: ${String(e?.message ?? e)}`);
    }
  }

  function apply(operation: Operation) {
    const image = imageRef.current;
    if (!image) return;
    try {
      const t0 = Date.now();
      operation.run(image);
      const t1 = Date.now();
      setRevision((r) => r + 1);
      const t2 = Date.now();
      setStatus(`${operation.label}: ${t1 - t0} ms, preview ${t2 - t1} ms`);
    } catch (e: any) {
      setStatus(`Failed: ${String(e?.message ?? e)}`);
    }
  }

  function save() {
    const image = imageRef.current;
    if (!image || !source) return;
    try {
      const started = Date.now();
      const outPath = `${toPath(source.uri).replace(/\.[^./]+$/, '')}-abra.jpg`;
      image.write(outPath);
      setStatus(`Saved ${outPath.split('/').pop()} in ${Date.now() - started} ms`);
    } catch (e: any) {
      setStatus(`Failed: ${String(e?.message ?? e)}`);
    }
  }

  return (
    <ScrollView contentContainerStyle={styles.container}>
      <Text style={styles.title}>Alakazam</Text>
      <FilePicker onPick={load} />
      {source && (
        <View style={styles.operations}>
          {OPERATIONS.map((operation) => (
            <Button key={operation.label} title={operation.label} onPress={() => apply(operation)} />
          ))}
          <Button title="Reset" color="#888" onPress={() => load(source)} />
          <Button title="Save" color="#2a7" onPress={save} />
        </View>
      )}
      {source && (
        <View style={styles.live}>
          {session.error && <Text style={styles.status}>{session.error}</Text>}
          <Slider label="Brightness" min={-100} max={100} reset triggerType="live" onTrigger={(v) => preview(EffectSpec.Brightness.new({ amount: Math.round(v) }))} />
          <Slider label="Contrast" min={-100} max={100} reset triggerType="live" onTrigger={(v) => preview(EffectSpec.Contrast.new({ amount: v }))} />
          <Slider label="Blur" min={0} max={30} reset triggerType="live" onTrigger={(v) => preview(EffectSpec.GaussianBlur.new({ radius: v }))} />
        </View>
      )}
      {status && <Text style={styles.status}>{status}</Text>}
      {source && VesselView && session.size && (
        <View style={[styles.preview, { width: previewWidth }]}>
          <VesselView source={session.view} style={fitSize(previewWidth, session.size)} />
        </View>
      )}
    </ScrollView>
  );
}

/** Fits a size inside the preview area, centered, so its pixels aren't stretched. */
function fitSize(p_width: number, p_size: { width: number; height: number }) {
  const fit = Math.min(p_width / p_size.width, PREVIEW_HEIGHT / p_size.height);
  const width = Math.max(1, p_size.width * fit);
  const height = Math.max(1, p_size.height * fit);
  return { position: 'absolute' as const, left: (p_width - width) / 2, top: (PREVIEW_HEIGHT - height) / 2, width, height };
}

const styles = StyleSheet.create({
  container: { padding: 16, gap: 12 },
  title: { fontSize: 24, fontWeight: 'bold' },
  operations: { flexDirection: 'row', flexWrap: 'wrap', gap: 8 },
  live: { gap: 8 },
  status: { fontSize: 12, color: '#444' },
  preview: { height: PREVIEW_HEIGHT, borderRadius: 12, backgroundColor: '#111', overflow: 'hidden' },
});
