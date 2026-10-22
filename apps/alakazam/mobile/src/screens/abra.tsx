import { autoTone, brightness, contrast, grayscale, invert } from '@abra/core/adjustments';
import { gaussianBlur, sharpen } from '@abra/core/filters';
import { type Image as AbraPixels, type ImageLike } from '@abra/core/image';
import { AbraImage } from '@alakazam/mobile';
import { useEffect, useRef, useState } from 'react';
import { Button, ScrollView, StyleSheet, Text, useWindowDimensions, View } from 'react-native';
import { FilePicker, type PickedImage } from '@/src/components/file-picker';
import { Slider } from '@/src/components/slider';
import { VesselView } from '@vessel/react-native';
import { useLiveSession } from '@/src/hooks/useLiveSession';

/** An edit made by a tool written against the library on a copy of the photo's pixels. */
type Operation = { label: string; tool: (photo: ImageLike) => void };

const OPERATIONS: Operation[] = [
  { label: 'Grayscale', tool: (photo) => grayscale().apply(photo) },
  { label: 'Invert', tool: (photo) => invert().apply(photo) },
  { label: 'Brightness +40', tool: (photo) => brightness(40).apply(photo) },
  { label: 'Auto tone', tool: (photo) => autoTone().apply(photo) },
  { label: 'Blur 8px', tool: (photo) => gaussianBlur(8).apply(photo) },
  { label: 'Sharpen', tool: (photo) => sharpen().apply(photo) },
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
  /** Shows what a tool makes of the photo over the preview. */
  const preview = (tool: (photo: ImageLike) => void) => {
    const image = imageRef.current;
    if (!image) return;
    const photo = image.clone();
    try {
      tool(photo);
      session.show(photo as AbraImage);
    } finally {
      (photo as AbraPixels).uniffiDestroy();
    }
  };

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
      const photo = image.clone();
      try {
        operation.tool(photo);
        image.replaceWith(photo);
      } finally {
        (photo as AbraPixels).uniffiDestroy();
      }
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
      image.write(outPath, undefined);
      setStatus(`Saved ${outPath.split('/').pop()} in ${Date.now() - started} ms`);
    } catch (e: any) {
      setStatus(`Failed: ${String(e?.message ?? e)}`);
    }
  }

  return (
    <ScrollView contentContainerStyle={styles.container}>
      <Text style={styles.title}>Alakazam</Text>
      <FilePicker onPick={assets => load(assets[assets.length - 1])} />
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
          <Slider label="Brightness" min={-100} max={100} reset triggerType="live" onTrigger={(v) => preview((photo) => brightness(Math.round(v)).apply(photo))} />
          <Slider label="Contrast" min={-100} max={100} reset triggerType="live" onTrigger={(v) => preview((photo) => contrast(v).apply(photo))} />
          <Slider label="Blur" min={0} max={30} reset triggerType="live" onTrigger={(v) => preview((photo) => gaussianBlur(v).apply(photo))} />
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
