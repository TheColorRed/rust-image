import { isOnProjectCanvas, mouseDown$, mouseMoveWithLeftButtonDown$, mouseUp$ } from '@/events/body';
import { projectCursor } from '@/events/project';
import { toSvgCursor } from '@/lib/cursor';
import { cn } from '@/lib/util';
import { setTheActiveColor } from '@/state/color';
import { Option } from '@/ui/option';
import { Select } from '@/ui/select';
import { faEyedropper } from '@fortawesome/sharp-solid-svg-icons';
import { useEffect, useRef, useState } from 'react';
import { from, map, merge, switchMap, tap } from 'rxjs';

export function EyeDropperTool() {
  const [sampleMode, setSampleMode] = useState('point');
  const [sampleColor, setSampleColor] = useState<string | null>(null);
  const [firstSampleColor, setFirstSampleColor] = useState<string | null>(null);
  const [samplePosition, setSamplePosition] = useState<[number, number]>([0, 0]);
  const [sampleVisible, setSampleVisible] = useState(false);
  const [sampleWhat, setSampleWhat] = useState<'all' | 'current-layer'>('all');

  const sampleColorRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const cursor = toSvgCursor(faEyedropper);
    projectCursor.next({ cursor, origin: [0, 16], size: 16 });

    const startSamplingSub = merge(mouseDown$, mouseMoveWithLeftButtonDown$)
      .pipe(
        isOnProjectCanvas(),
        switchMap(event => {
          // Sample color from canvas at mouse position
          const canvas = event.target as HTMLCanvasElement;
          const { left, top } = canvas.getBoundingClientRect();
          const x = event.clientX - left;
          const y = event.clientY - top;
          const [width, height] = sampleMode === 'point' ? [1, 1] : sampleMode.split('x').map(Number);

          setSamplePosition([x, y]);
          setSampleVisible(true);

          // Center the width/height around the cursor position
          const sampleX = Math.floor(x - width / 2);
          const sampleY = Math.floor(y - height / 2);

          return from(window.alakazam.projects.getActiveProjectMetadata()).pipe(
            switchMap(metadata => window.alakazam.projects.getActiveLayers(metadata!.id)),
            map(layers => (sampleWhat === 'current-layer' ? layers[0]?.id : undefined)),
            switchMap(layer =>
              window.alakazam.imageData.sampleColor(sampleX, sampleY, width, height, sampleWhat, layer),
            ),
            tap(color => event.type === 'mousedown' && setFirstSampleColor(color)),
            tap(color => setSampleColor(color)),
            tap(color => setTheActiveColor(color)),
          );
        }),
      )
      .subscribe();

    const stopSamplingSub = mouseUp$.subscribe(() => {
      setSampleVisible(false);
    });

    return () => {
      startSamplingSub.unsubscribe();
      stopSamplingSub.unsubscribe();
    };
  }, [sampleMode, sampleWhat]);

  return (
    <div>
      {sampleColor && (
        <div ref={sampleColorRef}>
          <div
            className={cn('pointer-events-none absolute z-10 h-10 w-10 border-4', sampleVisible ? 'block' : 'hidden')}
            style={{
              top: samplePosition[1] + 0,
              left: samplePosition[0] + 60,
              backgroundColor: sampleColor,
            }}
          />
          <div
            className={cn('pointer-events-none absolute z-10 h-10 w-10 border-4', sampleVisible ? 'block' : 'hidden')}
            style={{
              top: samplePosition[1] + 90,
              left: samplePosition[0] + 60,
              backgroundColor: firstSampleColor ?? '',
            }}
          />
        </div>
      )}
      <div className="flex gap-4">
        <Select value={sampleMode} onChange={setSampleMode} className="w-46">
          <Option value="point">Point Sample</Option>
          <Option value="3x3">3x3 Average</Option>
          <Option value="5x5">5x5 Average</Option>
          <Option value="11x11">11x11 Average</Option>
          <Option value="31x31">31x31 Average</Option>
          <Option value="51x51">51x51 Average</Option>
          <Option value="101x101">101x101 Average</Option>
        </Select>
        <Select value={sampleWhat} onChange={setSampleWhat} className="ml-4 w-60">
          <Option value="all">Sample All Layers</Option>
          <Option value="current-layer">Sample Current Layer</Option>
        </Select>
      </div>
    </div>
  );
}
