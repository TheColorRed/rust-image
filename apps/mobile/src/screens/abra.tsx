import { AbraImage, EffectSpec, type AbraImageLike } from '@alakazam/mobile';
import { AlphaType, Canvas, ColorType, Image as SkiaImage, Skia, type SkImage } from '@shopify/react-native-skia';
import { useEffect, useRef, useState } from 'react';
import { Button, PixelRatio, ScrollView, StyleSheet, Text, useWindowDimensions, View } from 'react-native';
import FilePicker, { type PickedImage } from '@/src/components/file-picker';
import { Slider } from '@/src/components/slider';
import { disposeLater, useLivePreview } from '@/src/hooks/useLivePreview';

type Operation = { label: string; run: (image: AbraImageLike) => void };

const OPERATIONS: Operation[] = [
  { label: 'Grayscale', run: (image) => image.grayscale() },
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
  const [preview, setPreview] = useState<SkImage | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  // The working image lives in Rust at full resolution; only screen-sized previews cross into JS.
  const imageRef = useRef<AbraImage | null>(null);
  const [revision, setRevision] = useState(0);
  const [liveOn, setLiveOn] = useState(false);
  const live = useLivePreview(
    source ? imageRef.current : null,
    Math.round(previewWidth * PixelRatio.get()),
    Math.round(PREVIEW_HEIGHT * PixelRatio.get()),
    revision,
  );

  // The JS garbage collector can't see the Rust-side pixels behind a handle, so free them explicitly.
  useEffect(() => () => imageRef.current?.uniffiDestroy(), []);

  /** Replaces the Skia preview with a screen-sized copy of the working image. */
  function refreshPreview() {
    const image = imageRef.current;
    if (!image) return;
    const scale = PixelRatio.get();
    const pixels = image.preview(Math.round(previewWidth * scale), Math.round(PREVIEW_HEIGHT * scale));
    const next = Skia.Image.MakeImage(
      { width: pixels.width, height: pixels.height, colorType: ColorType.RGBA_8888, alphaType: AlphaType.Unpremul },
      Skia.Data.fromBytes(new Uint8Array(pixels.data)),
      pixels.width * 4,
    );
    setPreview((previous) => {
      disposeLater(previous);
      return next;
    });
  }

  function load(asset: PickedImage) {
    try {
      const started = Date.now();
      imageRef.current?.uniffiDestroy();
      imageRef.current = AbraImage.read(toPath(asset.uri)) as AbraImage;
      refreshPreview();
      const image = imageRef.current;
      setSource(asset);
      setRevision((r) => r + 1);
      setLiveOn(false);
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
      refreshPreview();
      setRevision((r) => r + 1);
      setLiveOn(false);
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
          <Button
            title={liveOn ? 'Live preview: on' : 'Live preview: off'}
            onPress={() => {
              if (liveOn) live.clear();
              setLiveOn(!liveOn);
            }}
          />
          {liveOn && (
            <>
              <Text style={styles.status}>{live.isGpu ? 'Rendering on the GPU' : 'Rendering on the CPU'}</Text>
              {live.error && <Text style={styles.status}>{live.error}</Text>}
              <Slider label="Brightness" min={-100} max={100} reset triggerType="live" onTrigger={(v) => live.apply(EffectSpec.Brightness.new({ amount: Math.round(v) }))} />
              <Slider label="Contrast" min={-100} max={100} reset triggerType="live" onTrigger={(v) => live.apply(EffectSpec.Contrast.new({ amount: v }))} />
              <Slider label="Blur" min={0} max={30} reset triggerType="live" onTrigger={(v) => live.apply(EffectSpec.GaussianBlur.new({ radius: Math.round(v) }))} />
              <Slider
                label="Gradient angle"
                min={0}
                max={360}
                reset
                triggerType="live"
                onTrigger={(v) =>
                  live.apply(
                    EffectSpec.LinearGradient.new({
                      stops: [
                        { position: 0, r: 255, g: 64, b: 0, a: 255 },
                        { position: 1, r: 0, g: 96, b: 255, a: 255 },
                      ],
                      angle: v,
                      opacity: 0.6,
                    }),
                  )
                }
              />
            </>
          )}
        </View>
      )}
      {status && <Text style={styles.status}>{status}</Text>}
      {preview && (
        <Canvas style={[styles.preview, { width: previewWidth }]}>
          <SkiaImage image={liveOn && live.frame ? live.frame : preview} x={0} y={0} width={previewWidth} height={PREVIEW_HEIGHT} fit="contain" />
        </Canvas>
      )}
    </ScrollView>
  );
}

const styles = StyleSheet.create({
  container: { padding: 16, gap: 12 },
  title: { fontSize: 24, fontWeight: 'bold' },
  operations: { flexDirection: 'row', flexWrap: 'wrap', gap: 8 },
  live: { gap: 8 },
  status: { fontSize: 12, color: '#444' },
  preview: { height: PREVIEW_HEIGHT, borderRadius: 12, backgroundColor: '#111' },
});
